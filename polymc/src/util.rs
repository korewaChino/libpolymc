use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Uri};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::convert::Infallible;
/// Utility functions and objects for libpolymc
use std::env;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;

lazy_static! {
    /// Channel used to send shutdown signal - wrapped in an Option to allow
    /// it to be taken by value (since oneshot channels consume themselves on
    /// send) and an Arc<Mutex> to allow it to be safely shared between threads
    static ref SHUTDOWN_TX: Arc<Mutex<Option<Sender<()>>>> = <_>::default();

    // mutable string to store the access token
    static ref TOKEN: Arc<Mutex<Option<Uri>>> = <_>::default();
}

pub fn get_dir(sub: &str) -> String {
    //TODO: Change this back to home dir

    dotenv::dotenv().ok();
    // get environment variable called "POLYMC_DIR"

    if let Ok(dir) = env::var("POLYMC_DIR") {
        // if it exists, return it
        return dir;
    } else{
        //let mut dir = dirs::data_dir().unwrap();
        //dir.push("polymc");

        // For production, comment below and uncomment above
        let mut dir = env::current_dir().unwrap();
        dir.push("test");

        dir.push(sub);
        dir.display().to_string()
    }


}

pub fn main_dir() -> String {
    //TODO: Change this back to home dir
    //let mut dir = dirs::current_dir.unwrap();
    // current dir
    let mut dir = env::current_dir().unwrap();
    //dir.push("polymc");
    dir.push("test");
    dir.display().to_string()
}

#[derive(Deserialize, Debug)]
pub struct Query {
    pub code: String,
    pub state: String,
}

async fn handle_queries(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // print the request
    //println!("{:?}", _req.uri());
    // Parse the query
    let query = _req.uri();
    // This is a very hacky and unsafe way to do this.
    //
    TOKEN.lock().await.replace(query.clone());

    //println!("{:#?}", query);
    if let Some(tx) = SHUTDOWN_TX.lock().await.take() {
        let _ = tx.send(());
    }

    // Send a response that automatically closes the tab
    Ok(Response::new(Body::from("<script>window.close()</script>")))
    //Ok(Response::new(format!("{:#?}", _req).into()))
}
#[allow(unused_must_use)] // graceful.wait is used to wait for requests
pub async fn fetch_queries(port: u16) -> Query {
    // Credits to:
    // https://stackoverflow.com/questions/63599177/how-do-i-terminate-a-hyper-server-after-fulfilling-one-request

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    SHUTDOWN_TX.lock().await.replace(tx);

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_queries)) });

    let addr = ([127, 0, 0, 1], port).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();
    });
    graceful.await;
    // Parse this query
    let query = TOKEN.lock().await.clone();
    let q: Query = serde_urlencoded::from_str(&query.unwrap().query().unwrap()).unwrap();
    q
}
