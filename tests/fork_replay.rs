//! Public mainnet corpus replay through the streamer facade and its account bridge.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_streamer_sdk::streaming::{event_parser::Protocol, grpc::AccountPretty};
use solana_streamer_sdk::{parse_encoded_rpc_transaction_as_streamer_events, sdk_bridge};
use std::str::FromStr;
fn corpus() -> Value {
    serde_json::from_str(include_str!("../validation/fixtures/mainnet-2026-09-25.json")).unwrap()
}
fn oracle() -> Value {
    serde_json::from_str(include_str!("../validation/fixtures/mainnet-2026-09-25.expected.json"))
        .unwrap()
}
fn assert_fields(actual: &Value, fields: &Value, context: &str) {
    for (key, expected) in fields.as_object().unwrap() {
        let actual = &actual[key];
        if actual.is_number() && expected.is_string() {
            assert_eq!(actual.to_string(), expected.as_str().unwrap(), "{context}: {key}");
        } else {
            assert_eq!(actual, expected, "{context}: {key}");
        }
    }
}

#[test]
fn recent_swaps_preserve_official_fields_through_streamer() {
    let c = corpus();
    let e = oracle();
    for (tx, wanted) in
        c["transactions"].as_array().unwrap().iter().zip(e["transactions"].as_array().unwrap())
    {
        let raw = serde_json::from_value(tx["encoded"].clone()).unwrap();
        let events = parse_encoded_rpc_transaction_as_streamer_events(
            &raw,
            0,
            &[Protocol::PumpFun, Protocol::PumpSwap],
            None,
        )
        .unwrap();
        let actual: Vec<Value> = events.iter().map(|e| serde_json::to_value(e).unwrap()).collect();
        for expectation in wanted["events"].as_array().unwrap() {
            let name = match expectation["kind"].as_str().unwrap() {
                "PumpFunBuy" | "PumpFunSell" => "PumpFunTradeEvent",
                "PumpSwapBuy" => "PumpSwapBuyEvent",
                "PumpSwapSell" => "PumpSwapSellEvent",
                _ => unreachable!(),
            };
            let event = actual
                .iter()
                .filter_map(|v| v.get(name))
                .find(|v| v["user"] == expectation["fields"]["user"])
                .unwrap_or_else(|| panic!("missing {name}: {actual:?}"));
            assert_fields(event, &expectation["fields"], tx["signature"].as_str().unwrap());
            assert_eq!(event["metadata"]["slot"], tx["encoded"]["slot"]);
        }
    }
}
#[test]
fn account_bridge_accepts_historical_and_padded_allocations() {
    let c = corpus();
    let e = oracle();
    for expected in e["accounts"].as_array().unwrap() {
        let fixture = c["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["address"] == expected["address"])
            .unwrap();
        let account = AccountPretty {
            slot: fixture["context"]["slot"].as_u64().unwrap(),
            signature: Signature::default(),
            pubkey: Pubkey::from_str(fixture["address"].as_str().unwrap()).unwrap(),
            owner: Pubkey::from_str(fixture["value"]["owner"].as_str().unwrap()).unwrap(),
            executable: false,
            lamports: fixture["value"]["lamports"].as_u64().unwrap(),
            rent_epoch: 0,
            data: STANDARD.decode(expected["data"].as_str().unwrap()).unwrap(),
            recv_us: 0,
        };
        let event = sdk_bridge::parse_account_event(
            &account,
            &[Protocol::PumpFun, Protocol::PumpSwap],
            None,
        )
        .expect("account bridge event");
        assert_eq!(event.metadata().slot, account.slot);
        let value = serde_json::to_value(event).unwrap();
        let payload = value.as_object().unwrap().values().next().unwrap();
        let field = if expected["kind"] == "curve" { "bonding_curve" } else { "pool" };
        let mut fields = expected["fields"].clone();
        // The streamer's documented API normalizes the zero quote mint to its SOL sentinel.
        if expected["kind"] == "curve"
            && fields["quote_mint"] == serde_json::json!([0; 32].to_vec())
        {
            fields["quote_mint"] =
                serde_json::json!(Pubkey::from_str("So11111111111111111111111111111111111111111")
                    .unwrap()
                    .to_bytes()
                    .to_vec());
        }
        assert_fields(
            &payload[field],
            &fields,
            &format!("{} {} bytes", expected["kind"], expected["length"]),
        );
    }
}
