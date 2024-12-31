use rocket::get;

#[get("/hello")]
pub fn hello() -> String {
    "hello".into()
}
