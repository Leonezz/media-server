// use rocket::{FromForm, get};

// #[derive(Debug, FromForm)]
// #[form(lenient)]
// pub struct TestForm {
//     #[field(name = "param")]
//     _optional_param: Option<bool>,
//     #[field(name = "param2")]
//     _param2: u64,
// }

use axum::extract::Query;

// #[get("/hello?<param..>")]
pub async fn hello(param: Query<String>) -> &'static str {
    tracing::info!("param: {:?}", param);
    "hello"
}
