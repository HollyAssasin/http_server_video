use axum::extract::{Path, State};
use axum::http::header::TRANSFER_ENCODING;
use axum::http::StatusCode;
use axum::Router;
use axum::routing::{get};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::Url;
use crate::state::SharedState;
use crate::utils::LargeChunkReaderStream;

/*
 All test methods assume the file is on your, not sent through the body with the request.
 */


pub fn get_test_routes() -> Router<SharedState> {
    Router::new()
        .route("/test/*file_name", get(receive_resource_test).post(send_resource_test).delete(delete_resource_test)) // endpoints for testing
}

// Testing method, that should we used with associated test POST method
async fn delete_resource_test(Path(file_name): Path<String>) -> StatusCode {
    let base = Url::parse( "http://127.0.0.1:3000/").unwrap();
    let mut futures = FuturesUnordered::new();
    let times = 1000;
    for i in 0..times {
        let file_name_clone = file_name.clone();
        let fin_name = format!("{}-{}", file_name_clone, i);
        let base_clone = base.clone();

        futures.push(tokio::spawn(async move {
            // let mut local_vec: Vec<Bytes> = Vec::new(); // uncomment to load the files, otherwise run out of memory for large files
            let client = reqwest::Client::new();
            let _res = client.delete(base_clone.join(&fin_name).unwrap())
                .send()
                .await.unwrap();

        }));
    }

    while futures.next().await.is_some() {} // Wait for all futures
    tracing::info!("FINISHED DELETING");
    StatusCode::OK
}

// Testing endpoint, sends file multiple times as a stream
async fn send_resource_test(Path(file_name): Path<String>) -> StatusCode {

    let base = Url::parse( "http://127.0.0.1:3000/").unwrap();
    let mut futures = FuturesUnordered::new();
    let times = 1000;
    for i in 0..times {
        let file_name_clone = file_name.clone();
        let fin_name = format!("{}-{}", file_name_clone, i);
        let base_clone = base.clone();

        futures.push(tokio::spawn(async move {
            let ababa = LargeChunkReaderStream::new(&file_name_clone, 4096).await.unwrap(); // Custom file reader
            let client = reqwest::Client::new();
            let _res = client.post(base_clone.join(&fin_name).unwrap())
                .header(TRANSFER_ENCODING, "chunked")
                .body(reqwest::Body::wrap_stream(ababa))
                .send()
                .await.unwrap();

        }));
    }

    while futures.next().await.is_some() {} // Wait for futures
    tracing::info!("FINISHED SENDING");
    StatusCode::OK
}

// Test method to consume immediately consume streams
async fn receive_resource_test(State(v_file): State<SharedState>, Path(file_name): Path<String>) -> StatusCode {
    let base = Url::parse("http://127.0.0.1:3000/").unwrap();
    let mut futures = FuturesUnordered::new();
    let times = 100;
    for i in 0..times {
        let file_name_clone = format!("{}-{}",file_name.clone(), i);
        let base_clone = base.clone();
        let lock = v_file.clone();

        futures.push(tokio::spawn(async move {
            // let mut local_vec: Vec<Bytes> = Vec::new(); // uncomment to load the files, careful about memory
            let mut vec_size = 0;
            let mut stream = reqwest::get(base_clone.join(&file_name_clone).unwrap())
                .await.unwrap().bytes_stream();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(item) => {
                        // local_vec.push(item);
                        vec_size += item.len();
                    },
                    Err(_) => tracing::error!("Error receiving chunk"),
                }
            }

            let my_size = lock.read().unwrap().file_system.get(&file_name_clone).unwrap().read().unwrap().video_file.iter().fold(0, |acc, x| acc + x.len());
            if my_size == vec_size {
                tracing::info!("Vectors are equal: {}", file_name_clone);
            } else {
                tracing::info!("VECTORS arent equal {:?} - {:?} -~- {}", my_size, vec_size, file_name_clone);
            }
        }));
    }

    while futures.next().await.is_some() {} // Wait for futures
    tracing::info!("Finished receiving file {} times", times);
    StatusCode::OK
}
