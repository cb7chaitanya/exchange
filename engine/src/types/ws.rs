use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "V")]
    pub volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<String>,
    pub id: i64,
    pub e: String, // Will always be "ticker"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<Vec<[String; 2]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<Vec<[String; 2]>>,
    pub e: String, // Will always be "depth"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub e: String, // Will always be "trade"
    pub t: i64,
    pub m: bool,
    pub p: String,
    pub q: String,
    pub s: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsMessageData {
    Ticker(TickerData),
    Depth(DepthData),
    Trade(TradeData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub stream: String,
    pub data: WsMessageData,
}
