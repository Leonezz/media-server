use rocket::{FromForm, get};

#[derive(Debug, FromForm)]
#[form(lenient)]
pub struct TestForm {
    #[field(name = "param")]
    _optional_param: Option<bool>,
    #[field(name = "param2")]
    _param2: u64,
}

#[get("/hello?<param..>")]
pub fn hello(param: Option<TestForm>) -> String {
    log::info!("param: {:?}", param);
    "hello".into()
}
