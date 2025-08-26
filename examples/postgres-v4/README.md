# Spin Outbound PostgreSQL example

This example shows how to access a PostgreSQL database from a Spin component.
It shows the new PostgreSQL range support in the v4 interface.

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

Curl with a year between 2005 and today as the path:

```
$ curl -i localhost:3000/2016
HTTP/1.1 200 OK
transfer-encoding: chunked
date: Mon, 18 Aug 2025 05:02:29 GMT

Splodge and Fang and Kiki and Slats
```
