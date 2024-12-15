pub fn concat_stream_id(stream_name: &str, app: &str) -> String {
    let mut res = app.to_owned();
    res.push_str("/");
    res.push_str(stream_name);
    res
}
