# axum-router

This example shows how to use [axum](https://github.com/tokio-rs/axum) with the `spin-sdk`

```
spin up --build 
curl --json '{"username": "jiggs"}' localhost:3000/users
{"id":1337,"username":"jiggs"}  
```