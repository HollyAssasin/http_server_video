# Video Optimized HTTP Server
A Rust based web server designed for efficient video streaming
and storing\
Utilizes Axum as its framework, providing endpoints for
uploading, streaming, listing and deleting videos.
Trie data-structure is used for storage.

Made for CDN77 as a trial task for rust-based position.

## Features
- **Transfer-Encoding: chunked** - Supports streaming video content
as soon as the first bytes are uploaded to the server.
- **Managing videos** - Allows uploading, listing, deleting and retrieving using
REST-like endpoints.
- **In-Memory Storage** - Uses Trie for fast and efficient storage and prefix search.
- **Concurrent Processing** - Uses Tokio's capabilities to process multiple video streams at once.

## Getting started
### Prerequisites
- Rust and Cargo should be installed on your machine.
### Installation
1. Clone the repository:
    ```
    git clone https://github.com/HollyAssasin/http_server_video.git
    cd http_client_opt
    ```
2. Build the project:
    ```
   cargo build --release
   ```
   2.1 For linux you might need OpenSSL installed, for Reqwest to work.
3. Run the server:
   ```
   cargo run --release
   ```

## Usage
By default, after starting the server it listens on `http://localhost:3000` for requests.
The server supports the following endpoints:
### Main endpoints
- `POST /{file_name}`: Saves the POST'ed video to server.
- `GET /{file_name}`: Sends the requested video file back as a stream.
- `LIST /{prefix}`: Custom HTTP method that returns currently stored video information as a JSON list.
- `DELETE /{file_name}`: Deletes a video from memory.
### Test endpoints (only for testing)
Assumes that the file is on the server, requires file path only.
- `POST /test/{file_name}?times={n}`: Uploads the file `n`-times or 1000 if not specified. Appends
the number of the file to the end of the file name.
- `GET /test/{file_name}??times={n}`: Compares the bytes of the file stored on the server and file received
from stream for `n` files, default 100.
- `DELETE /test/{file_name}??times={n}`: Deletes the first `n` files from memory, default 1000.

### Examples
- Get the file named `small_video.mp4`: `GET http://localhost:3000/small_video.mp4`
- Saves to memory the file named `big_video.webm`: `POST http://localhost:3000/big_video.webm`
- List all files located at address `/streams/*`: `LIST http://localhost:3000/streams/*`
- Deletes the file named `streams/medium_video.mp4`: `DELETE http://localhost:3000/streams/medium_video.mp4`
- Uploads the file `tiny.mp4` 1234 times: `POST http://localhost:3000/test/tiny.mp4?times=1234`

## Project Structure
- `main.rs`: Entry point of the application, more routes can be added here.
- `state.rs`: Global app state of the application, functions for processing video can be here.
- `stream_handlers`: Routes and handlers for stream handling are here.
- `test_handlers`: Routes and handlers for different testing lie here.
- `utils`: Different utilities, for example, custom async stream file reader.
   
