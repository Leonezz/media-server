use rocket::{
    FromForm,
    form::{self, Lenient},
    get,
};

#[derive(Debug, FromForm)]
#[form(lenient)]
pub struct TestForm {
    #[field(name = "param")]
    optional_param: Option<bool>,
    #[field(name = "param2")]
    param2: u64,
}

#[get("/hello?<param..>")]
pub fn hello(param: Option<TestForm>) -> String {
    tracing::info!("param: {:?}", param);
    "hello".into()
}
