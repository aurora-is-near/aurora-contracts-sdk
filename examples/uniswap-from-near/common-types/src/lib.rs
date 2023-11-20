use near_sdk::json_types::U64;

/// Input parameters for uniswap's `exact_output_single` function.
/// The types have been modified to optimize for JSON serialization.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializableExactOutputSingleParams {
    pub token_in: String,
    pub token_out: String,
    #[serde(default)]
    pub fee: Option<U64>,
    pub recipient: String,
    #[serde(default)]
    pub deadline: Option<String>,
    pub amount_out: String,
    #[serde(default)]
    pub amount_in_max: Option<String>,
    #[serde(default)]
    pub price_limit: Option<String>,
}
