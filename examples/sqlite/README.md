# Spin SQLite component in Rust

A simple Spin HTTP application that demonstrates the SQLite SDK. It exposes three
routes for initializing a database, inserting users, and querying them back.

## Build and run

```shell
$ RUST_LOG=spin=trace spin build --up
```

The application will be available on `http://localhost:3000`.

## Usage

Initialize the database (creates the `users` table):

```shell
$ curl -i -X POST localhost:3000/init
HTTP/1.1 200 OK
content-length: 20

Database initialized
```

Create a user:

```shell
$ curl -i -X POST "localhost:3000/users?name=Alice&email=alice@example.com"
HTTP/1.1 201 Created
content-length: 22

Created user with id 1
```

```shell
$ curl -i -X POST "localhost:3000/users?name=Bob&email=bob@example.com"
HTTP/1.1 201 Created
content-length: 22

Created user with id 2
```

List all users:

```shell
$ curl -s localhost:3000/users | jq .
[
  {
    "id": 1,
    "name": "Alice",
    "email": "alice@example.com"
  },
  {
    "id": 2,
    "name": "Bob",
    "email": "bob@example.com"
  }
]
```
