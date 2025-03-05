/// Currently only WsReverse is supported. I do not intend to implement more but PRs are welcome.
#[derive(Clone)]
pub struct ReverseConnectionConfig {
    pub target: String,
    pub auth: Option<String>,
}
