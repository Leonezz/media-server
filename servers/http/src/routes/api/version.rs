use rocket::{get, serde::json::Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerVersionResponse {
    version: String,
}

#[get("/version")]
pub fn get_version() -> Json<ServerVersionResponse> {
    Json(ServerVersionResponse {
        version: "this is a dump version".to_string(),
    })
}
