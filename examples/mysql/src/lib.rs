use anyhow::{Result, anyhow};
use futures::SinkExt;
use http::{HeaderValue, Method};
use spin_sdk::{
    http::{
        BoxBody, EmptyBody, FullBody, IntoResponse, Request, Response, body::IncomingBodyExt,
        box_body,
    },
    http_service,
    mysql_async::{self as mysql, ParameterValue},
};
use std::{collections::HashMap, str::FromStr};

use crate::model::as_pet;

mod model;

// The environment variable set in `spin.toml` that points to the
// address of the MySQL server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

enum RequestAction {
    List,
    Get(i32),
    Create(String, Option<String>, bool),
    Error(u16),
}

#[http_service]
async fn rust_outbound_mysql(req: Request) -> Result<impl IntoResponse> {
    let response = match parse_request(req).await? {
        RequestAction::List => list().await?,
        RequestAction::Get(id) => get(id).await?,
        RequestAction::Create(name, prey, is_finicky) => create(name, prey, is_finicky).await?,
        RequestAction::Error(status) => error(status)?,
    };
    Ok(response)
}

async fn parse_request(req: Request) -> Result<RequestAction> {
    match *req.method() {
        Method::GET => match req.headers().get("spin-path-info") {
            None => Ok(RequestAction::Error(500)),
            Some(header_val) => match header_val_to_int(header_val) {
                Ok(None) => Ok(RequestAction::List),
                Ok(Some(id)) => Ok(RequestAction::Get(id)),
                Err(()) => Ok(RequestAction::Error(404)),
            },
        },
        Method::POST => {
            let body = req.into_body().bytes().await?;
            let map: HashMap<String, String> = serde_json::from_slice(&body)?;

            // let map = req.body();
            let name = match map.get("name") {
                Some(n) => n.to_owned(),
                None => return Ok(RequestAction::Error(400)), // If this were a real app it would have error messages
            };
            let prey = map.get("prey").cloned();
            let is_finicky = map
                .get("is_finicky")
                .map(|s| s == "true")
                .unwrap_or_default();
            Ok(RequestAction::Create(name, prey, is_finicky))
        }
        _ => Ok(RequestAction::Error(405)),
    }
}

fn header_val_to_int(header_val: &HeaderValue) -> Result<Option<i32>, ()> {
    match header_val.to_str() {
        Ok(path) => {
            let path_parts = &(path.split('/').skip(1).collect::<Vec<_>>()[..]);
            match *path_parts {
                [""] => Ok(None),
                [id_str] => match i32::from_str(id_str) {
                    Ok(id) => Ok(Some(id)),
                    Err(_) => Err(()),
                },
                _ => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

async fn list() -> Result<Response<BoxBody>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address).await?;

    let sql = "SELECT id, name, prey, is_finicky FROM pets";
    let mut qr = conn.query(sql, &[]).await?;

    let (mut tx, body) = spin_sdk::http::body::stream();

    spin_sdk::wasip3::spawn(async move {
        let column_summary = qr
            .columns()
            .iter()
            .map(format_col)
            .collect::<Vec<_>>()
            .join(", ");

        let mut pet_count = 0;

        while let Some(row) = qr.next().await {
            let pet = as_pet(&row).ok_or(anyhow!("un-decodable entry"));
            println!("{:#?}", pet);
            tx.send(format!("{:#?}\n", pet)).await.unwrap(); // caller has gone away
            pet_count += 1;
        }

        match qr.result().await {
            Ok(()) => {
                tx.send(format!("{pet_count} pets found\n")).await.unwrap();
                tx.send(format!("Column info: {column_summary}\n"))
                    .await
                    .unwrap();
            }
            Err(e) => {
                tx.send(format!("List failed! {e:#}\n")).await.unwrap();
            }
        }
    });

    Ok(Response::new(box_body(body)))
}

async fn get(id: i32) -> Result<Response<BoxBody>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address).await?;

    let sql = "SELECT id, name, prey, is_finicky FROM pets WHERE id = ?";
    let mut qr = conn.query(sql, &[id.into()]).await?;

    let response = match qr.next().await {
        None => Response::builder()
            .status(404)
            .body(box_body(EmptyBody::new()))?,
        Some(row) => {
            let pet = as_pet(&row);
            let message = format!("{:?}\n", pet);
            Response::builder()
                .status(200)
                .body(box_body(FullBody::new(message.into())))?
        }
    };

    Ok(response)
}

async fn create(name: String, prey: Option<String>, is_finicky: bool) -> Result<Response<BoxBody>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address).await?;

    let id = max_pet_id(&conn).await? + 1;

    let prey_param = match prey {
        None => ParameterValue::DbNull,
        Some(str) => ParameterValue::Str(str),
    };

    let is_finicky_param = ParameterValue::Int8(i8::from(is_finicky));

    let sql = "INSERT INTO pets (id, name, prey, is_finicky) VALUES (?, ?, ?, ?)";
    let params = vec![
        ParameterValue::Int32(id),
        ParameterValue::Str(name),
        prey_param,
        is_finicky_param,
    ];
    conn.execute(sql, params).await?;

    let location_url = format!("/{}", id);

    Ok(http::Response::builder()
        .status(201)
        .header("Location", location_url)
        .body(box_body(EmptyBody::new()))?)
}

fn error(status: u16) -> Result<Response<BoxBody>> {
    Ok(http::Response::builder()
        .status(status)
        .body(box_body(EmptyBody::new()))?)
}

fn format_col(column: &mysql::Column) -> String {
    format!("{}: {:?}", column.name, column.data_type)
}

async fn max_pet_id(conn: &mysql::Connection) -> Result<i32> {
    let sql = "SELECT MAX(id) FROM pets";
    let mut qr = conn.query(sql, &[]).await?;

    match qr.rows().next().await {
        None => Ok(0),
        Some(row) => match row.first() {
            None => Ok(0),
            Some(mysql::DbValue::Int32(i)) => Ok(*i),
            Some(other) => Err(anyhow!(
                "Unexpected non-integer ID {:?}, can't insert",
                other
            )),
        },
    }
}
