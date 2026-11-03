use spin_sdk::{
    http::{body::IncomingBodyExt as _, IntoResponse, Request},
    http_service,
};

#[http_service]
async fn spawn_progress(req: Request) -> anyhow::Result<impl IntoResponse> {
    req.into_body().bytes().await?;
    let (reply, response) = futures::channel::oneshot::channel();
    #[allow(deprecated)]
    spin_sdk::wasip3::spawn(async move {
        spin_sdk::time::sleep(std::time::Duration::from_millis(1)).await;
        // After this resumed poll, only the newly spawned child can wake the receiver.
        let (sender, receiver) = futures::channel::oneshot::channel();
        spin_sdk::wasip3::spawn_local(async move {
            let _ = sender.send("spawned child completed");
        });
        let timer = Box::pin(spin_sdk::time::sleep(std::time::Duration::from_secs(30)));
        let message = match futures::future::select(receiver, timer).await {
            futures::future::Either::Left((message, _timer)) => message,
            futures::future::Either::Right(_) => {
                panic!("spawned child waited for an unrelated host timer")
            }
        };
        let _ = reply.send(message);
    });
    Ok(response.await??)
}
