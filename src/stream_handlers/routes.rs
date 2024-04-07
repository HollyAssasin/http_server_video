use std::sync::{Arc, RwLock};
use std::time::Duration;
use async_stream::stream;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::{Error, Json, Router};
use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, TRANSFER_ENCODING};
use axum::response::IntoResponse;
use axum::routing::any;
use bytes::Bytes;
use futures::StreamExt;
use radix_trie::TrieCommon;
use serde_json::{json, Value};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, sleep};
use crate::state::{SharedState, IndividualVideo};

pub fn get_stream_routes() -> Router<SharedState> {
    Router::new()
        .route("/*path", any(get_list).post(receive_resource).get(get_resource).delete(delete_resource))
}

// Deletes resource specified at file_name, if exists
async fn delete_resource(Path(file_name): Path<String>, State(v_file): State<SharedState>) -> StatusCode {
    let shared_state = v_file.clone();
    let video_file = { // Retrieve video file
        let hash_map_lock = shared_state.read().unwrap(); // Lock is only held in this block
        hash_map_lock.file_system.get(&file_name).is_some()
    };
    match video_file {
        false => { // No video
            StatusCode::NOT_FOUND
        },
        true => { // Can check if currently streaming, but assume that encoder/video handles this
            let mut hash_map_lock = shared_state.write().unwrap();
            hash_map_lock.file_system.remove(&file_name);
            StatusCode::OK
        }
    }
}

// Custom HTTP Method LIST, returns all videos in JSON format.
async fn get_list(Path(prefix): Path<String>, State(v_file): State<SharedState>, request: Request) -> Result<Json<Vec<Value>>, StatusCode> {
    match request.method().as_str() {
        "LIST" => {
            let file_sys_lock = v_file.read().unwrap();
            tracing::info!("LIST TO {}", prefix);
            let sub_trie = file_sys_lock.file_system.get_raw_descendant(prefix.trim_end_matches('*'));
            match sub_trie {
                Some(tree) => {
                    let mut jsons = Vec::new();
                    for (name, video_file_rw) in tree.iter() {
                        jsons.push(json!({
                            "name": name,
                            "content-type": video_file_rw.read().unwrap().content_type,
                            "streaming_to_memory": video_file_rw.read().unwrap().streaming
                        }));
                    }
                    if jsons.is_empty() {
                        return Err(StatusCode::NOT_FOUND);
                    }
                    Ok(Json(jsons))
                },
                None => {
                    Err(StatusCode::NOT_FOUND)
                }
            }
        },
        _ => Err(StatusCode::METHOD_NOT_ALLOWED)
    }

}

// Returns a Byte stream of a file named file_name, if exists
async fn get_resource(State(v_file): State<SharedState>, Path(file_name): Path<String>) -> impl IntoResponse {
    let shared_state = v_file.clone();
    let video_file = { // Retrieve video file
        let hash_map_lock = shared_state.read().unwrap(); // Lock is only held in this block
        hash_map_lock.file_system.get(&file_name).cloned()
    };
    return match video_file {
        None => { // No video
            StatusCode::NOT_FOUND.into_response()
        }
        Some(video_state_lock) => {
            let sender =  {
                let video_f_lock = video_state_lock.read().unwrap();
                video_f_lock.tx.is_some()
            };
            if sender { // Currently sending chunks
                let (byte_vec, mut receiver) = {
                    let video_file = video_state_lock.read().unwrap();
                    let receiver = video_file.tx.as_ref().unwrap().subscribe(); // We create it here to catch all the missed chunks.
                    (video_file.video_file.clone(), receiver)
                };
                let s = stream! {
                    // Return all chunks from memory first
                    for byte in byte_vec {
                        // tracing::info!("READING FROM MEMORY {:?}", byte.len());
                        yield Ok::<Bytes, Error>(byte);
                    }
                    
                    // Return all chunks from the broadcast channel associated with the file
                    loop {
                        let received = receiver.recv().await;
                        match received {
                            Ok(chunk) => {
                                // println!("Chunk from channel {:?}", chunk.len());
                                yield Ok::<Bytes, Error>(chunk)
                            }
                            Err(e) => match e {
                                // Impossible to handle in current configuration, will have to call
                                // GET again, in another call
                                RecvError::Lagged(_missed) => {
                                    tracing::error!("Missed messages");
                                    yield Err(Error::new("Missed messages from channel"));
                                    break;
                                },
                                // All messages successfully handled
                                RecvError::Closed => {
                                    tracing::info!("Channel closed");
                                    break;
                                },
                            }
                        }

                    }
                    drop(receiver);
                    tracing::info!("Finished sending stream");
                };
                Response::builder()
                    .status(StatusCode::OK)
                    .header(TRANSFER_ENCODING, "chunked") // not needed in axum
                    .body(Body::from_stream(s))
                    .unwrap()
            } else {
                // All chunks from memory only
                let byte_vec = {
                    let video_file = video_state_lock.read().unwrap();
                    video_file.video_file.clone()
                };
                let s = stream! {
                    for byte in byte_vec {
                        yield Ok::<Bytes, Error>(byte);
                    }
                };

                Response::builder()
                    .status(StatusCode::OK)
                    .header(TRANSFER_ENCODING, "chunked")// not needed in axum
                    .body(Body::from_stream(s))
                    .unwrap()
            }
        }
    }
}

// Handles POST method containing a stream of data. Saves to vector and broadcasts chunks, if 
// someone is listening
async fn receive_resource( State(v_file): State<SharedState>, Path(file_name): Path<String>, headers: HeaderMap, body: Body) -> StatusCode {
    tracing::info!("Receiving file with name: {:?}", file_name.clone());
    let mut stream = body.into_data_stream();
    let mut _last_time = Instant::now(); // for debugging
    let (tx, _rx) = tokio::sync::broadcast::channel::<Bytes>(5000); // Find some appropriate number, or use file hints, to not miss messages. For insta consume streams doesn't need to be big (1-10)
    drop(_rx); // We don't use it anywhere
    let content: String = match headers.get(CONTENT_TYPE) {
        Some(content_type) => {
            content_type.to_str().unwrap().parse().unwrap()
        },
        None => {
            String::from("octet-stream")
        }
    };
    v_file.write().unwrap().file_system.insert(file_name.clone(), Arc::new(RwLock::new(IndividualVideo{
        video_file: Vec::new(),
        tx: Some(tx),
        content_type: content,
        streaming: true,
    }))); // Create new video file

    while let Some(Ok(data)) = stream.next().await {
        // Process each chunk as it arrives
        let now = Instant::now();
        sleep(Duration::from_millis(1)).await; // Artificial slow down

        let fs_lock = v_file.read().unwrap();
        let video_lock = fs_lock.file_system.get(&file_name).unwrap();
        let mut to_write = video_lock.write().unwrap();
        tracing::debug!("Chunk received: {:?} bytes, With size {:?}.", to_write.video_file.len(),  data.len());
        to_write.video_file.push(data.clone());
        if to_write.tx.as_ref().is_some() && to_write.tx.as_ref().unwrap().receiver_count() > 0 {
            let resp = to_write.tx.as_ref().unwrap().send(data);
            match resp {
                Ok(_sent_size) => {
                    // tracing::debug!("Sent {:?}", _sent_size)
                },
                Err(e) => tracing::error!("Failed to send data: {:?}", e),
            }
            tracing::debug!("Currently {:?} receivers and queue of {:?}",to_write.tx.as_ref().unwrap().receiver_count(), to_write.tx.as_ref().unwrap().len());
        }
        _last_time = now;
    }
    let fs_lock = v_file.read().unwrap();
    let video_lock = fs_lock.file_system.get(&file_name).unwrap();
    let mut to_write = video_lock.write().unwrap();
    to_write.tx = None; // Drop sender to close the channel
    to_write.streaming = false;
    tracing::info!("Finished receiving file in post");
    StatusCode::OK
}
