pub mod route_guide {
    tonic::include_proto!("routeguide");
}

use route_guide::route_guide_server::RouteGuide;
use route_guide::route_guide_server::RouteGuideServer;

use futures::{SinkExt, Stream};
use spin_sdk::{
    sqlite::{self, Value},
    wasip3,
};
use tonic::{Request, Response, Status, Streaming};

#[spin_sdk::http_service]
async fn handle(req: spin_sdk::http::Request) -> impl spin_sdk::http::IntoResponse {
    spin_sdk::http::grpc::serve(RouteGuideServer::new(Svc), req).await
}

struct Svc;

#[tonic::async_trait]
impl RouteGuide for Svc {
    async fn get_feature(
        &self,
        request: Request<route_guide::Point>,
    ) -> Result<Response<route_guide::Feature>, Status> {
        let conn = sqlite::Connection::open_default()
            .await
            .map_err(as_status)?;

        let location = request.into_inner();

        let name = feature_name_at(&conn, location)
            .await
            .map_err(as_status)?
            .unwrap_or_default();

        let feat = route_guide::Feature {
            name,
            location: Some(location),
        };

        Ok(Response::new(feat))
    }

    type ListFeaturesStream = std::pin::Pin<
        Box<dyn Stream<Item = Result<route_guide::Feature, Status>> + Send + 'static>,
    >;

    async fn list_features(
        &self,
        request: Request<route_guide::Rectangle>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        // Helper function to let us use `?` syntax to manage errors.
        async fn list_impl(
            tx: &mut futures::channel::mpsc::Sender<Result<route_guide::Feature, Status>>,
            bounds: route_guide::Rectangle,
        ) -> Result<(), sqlite::Error> {
            let lat1 = bounds.lo.unwrap_or_default().latitude;
            let lat2 = bounds.hi.unwrap_or_default().latitude;
            let min_lat = std::cmp::min(lat1, lat2).into();
            let max_lat = std::cmp::max(lat1, lat2).into();

            let long1 = bounds.lo.unwrap_or_default().longitude;
            let long2 = bounds.hi.unwrap_or_default().longitude;
            let min_long = std::cmp::min(long1, long2).into();
            let max_long = std::cmp::max(long1, long2).into();

            let conn = sqlite::Connection::open_default().await?;

            let mut features_qr = conn.execute("SELECT lat, long, name FROM features WHERE lat >= ? AND lat <= ? AND long >= ? AND long <= ?", [Value::Integer(min_lat), Value::Integer(max_lat), Value::Integer(min_long), Value::Integer(max_long)]).await?;

            while let Some(feat_row) = features_qr.next().await {
                let latitude = feat_row.get::<i32>(0).unwrap_or_default();
                let longitude = feat_row.get::<i32>(1).unwrap_or_default();
                let name = feat_row
                    .get::<&str>(2)
                    .map(|s| s.to_owned())
                    .unwrap_or_default();

                let feat = route_guide::Feature {
                    name,
                    location: Some(route_guide::Point {
                        latitude,
                        longitude,
                    }),
                };

                if tx.send(Ok(feat)).await.is_err() {
                    break;
                }
            }

            features_qr.result().await?;

            Ok(())
        }

        let bounds = request.into_inner();

        let (mut tx, rx) = futures::channel::mpsc::channel(1024);

        wasip3::spawn(async move {
            if let Err(e) = list_impl(&mut tx, bounds).await {
                _ = tx.send(Err(as_status(e))).await;
            }
        });

        Ok(Response::new(Box::pin(rx)))
    }

    async fn record_route(
        &self,
        request: Request<Streaming<route_guide::Point>>,
    ) -> Result<Response<route_guide::RouteSummary>, Status> {
        let mut req = request;
        let r = req.get_mut();

        let mut distance = 0;
        let mut count = 0;
        let mut feature_count = 0;
        let mut last_pt = None;
        let start_time = std::time::SystemTime::now();

        let conn = sqlite::Connection::open_default()
            .await
            .map_err(as_status)?;

        loop {
            let Some(pt) = r.message().await? else {
                break;
            };

            count += 1;

            if let Some(last) = last_pt {
                distance += dist(last, pt);
            }
            if feature_name_at(&conn, pt).await.is_ok_and(|f| f.is_some()) {
                feature_count += 1;
            }

            last_pt = Some(pt);
        }

        let end_time = std::time::SystemTime::now();
        let elapsed_time = end_time
            .duration_since(start_time)
            .unwrap_or_default()
            .as_secs()
            .try_into()
            .unwrap_or_default();

        Ok(Response::new(route_guide::RouteSummary {
            point_count: count,
            feature_count,
            distance,
            elapsed_time,
        }))
    }

    type RouteChatStream = std::pin::Pin<
        Box<dyn Stream<Item = Result<route_guide::RouteNote, Status>> + Send + 'static>,
    >;

    async fn route_chat(
        &self,
        request: Request<Streaming<route_guide::RouteNote>>,
    ) -> Result<Response<Self::RouteChatStream>, Status> {
        // This operation is explained as: accept a stream of messages from the client; for
        // each message, respond with the prior messages at the same location.
        // (This means that the server never _initiates_ send messages. If it did, Spin could
        // handle that, but we'd need separate receive and send tasks running concurrently.
        // The routeguide sample doesn't have a scenario that shows this.)

        // Helper function to let us use `?` syntax to manage errors.
        async fn insert_and_reply(
            conn: &sqlite::Connection,
            tx: &mut futures::channel::mpsc::Sender<Result<route_guide::RouteNote, Status>>,
            message: route_guide::RouteNote,
        ) -> Result<(), sqlite::Error> {
            if let Some(location) = message.location {
                let lat = location.latitude.into();
                let long = location.longitude.into();
                let message = message.message;

                let ins_qr = conn
                    .execute(
                        "INSERT INTO route_notes(lat, long, msg_text) VALUES (?, ?, ?)",
                        [
                            Value::Integer(lat),
                            Value::Integer(long),
                            Value::Text(message),
                        ],
                    )
                    .await?;
                ins_qr.collect().await?;
                let to_skip = conn.last_insert_rowid().await;

                let mut notes_qr = conn.execute("SELECT lat, long, msg_text FROM route_notes WHERE lat = ? AND long = ? AND rowid <> ? ORDER BY seq_no", [Value::Integer(lat), Value::Integer(long), Value::Integer(to_skip)]).await?;

                while let Some(row) = notes_qr.next().await {
                    let prev_msg = route_guide::RouteNote {
                        location: Some(route_guide::Point {
                            latitude: row.get::<i32>(0).unwrap_or_default(),
                            longitude: row.get::<i32>(1).unwrap_or_default(),
                        }),
                        message: row.get::<&str>(2).map(|s| s.to_owned()).unwrap_or_default(),
                    };
                    if tx.send(Ok(prev_msg)).await.is_err() {
                        break;
                    }
                }

                notes_qr.result().await?;
            };

            Ok(())
        }

        let mut req_stm = request.into_inner();

        let conn = sqlite::Connection::open_default()
            .await
            .map_err(as_status)?;

        let (mut tx, rx) = futures::channel::mpsc::channel(1024);

        wasip3::spawn(async move {
            while let Ok(Some(message)) = req_stm.message().await {
                if let Err(e) = insert_and_reply(&conn, &mut tx, message).await {
                    _ = tx.send(Err(as_status(e))).await;
                    break;
                }
            }
        });

        Ok(tonic::Response::new(Box::pin(rx)))
    }
}

fn as_status(e: sqlite::Error) -> Status {
    Status::internal(e.to_string())
}

async fn feature_name_at(
    conn: &sqlite::Connection,
    location: route_guide::Point,
) -> Result<Option<String>, sqlite::Error> {
    let latitude = location.latitude.into();
    let longitude = location.longitude.into();

    let mut features_qr = conn
        .execute(
            "SELECT name FROM features WHERE lat = ? AND long = ?",
            [Value::Integer(latitude), Value::Integer(longitude)],
        )
        .await?;

    let name = match features_qr.next().await {
        None => {
            features_qr.result().await?; // check if the None was due to an error
            None
        }
        Some(row) => row.get::<&str>(0).map(|s| s.to_owned()),
    };

    Ok(name)
}

fn dist(pt1: route_guide::Point, pt2: route_guide::Point) -> i32 {
    let latd = pt1.latitude - pt2.latitude;
    let longd = pt1.longitude - pt2.longitude;
    let d = f64::sqrt((latd * latd + longd * longd) as f64);
    d as i32
}
