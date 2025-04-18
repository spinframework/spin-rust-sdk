# Spin Outbound PostgreSQL example

This example shows how to access a PostgreSQL database from Spin component.

## Prerequisite: Postgres

This example assumes postgres is running and accessible locally via its standard 5432 port.

We suggest running the `postgres` Docker container which has the necessary postgres user permissions
already configured. For example:

```
docker run --rm -h 127.0.0.1 -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres
```

## Spin up

Then, run the following from the root of this example:

```
createdb -h localhost -U postgres spin_dev
psql -h localhost -U postgres -d spin_dev -f db/testdata.sql
spin build --up
```

Curl the read route:

```
$ curl -i localhost:3000/read
HTTP/1.1 200 OK
transfer-encoding: chunked
date: Wed, 06 Nov 2024 20:17:03 GMT

Found 2 article(s) as follows:
article: Article {
    id: 1,
    title: "My Life as a Goat",
    content: "I went to Nepal to live as a goat, and it was much better than being a butler.",
    authorname: "E. Blackadder",
    published: Date(
        2024-11-05,
    ),
    coauthor: None,
}
article: Article {
    id: 2,
    title: "Magnificent Octopus",
    content: "Once upon a time there was a lovely little sausage.",
    authorname: "S. Baldrick",
    published: Date(
        2024-11-06,
    ),
    coauthor: None,
}

(Column info: id:DbDataType::Int32, title:DbDataType::Str, content:DbDataType::Str, authorname:DbDataType::Str, published:DbDataType::Date, coauthor:DbDataType::Str)
```

Curl the write route:

```
$ curl -i localhost:3000/write
HTTP/1.1 200 OK
content-length: 9
date: Sun, 25 Sep 2022 15:46:22 GMT

Count: 3
```

Curl the write_datetime_info route to experiment with date time types:
```
$ curl -i localhost:3000/write_datetime_info
HTTP/1.1 200 OK
content-length: 9
date: Sun, 25 Sep 2022 15:46:22 GMT

Count: 4
```

Read endpoint should now also show a row with publisheddate, publishedtime, publisheddatetime and readtime values.
