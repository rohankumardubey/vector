use crate::{
    config::{DataType, TransformConfig, TransformDescription},
    event::{Event, LookupBuf, Value},
    internal_events::{
        LogfmtParserConversionFailed, LogfmtParserEventProcessed, LogfmtParserMissingField,
    },
    transforms::{FunctionTransform, Transform},
    types::{parse_conversion_map, Conversion},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct LogfmtConfig {
    pub field: Option<LookupBuf>,
    pub drop_field: bool,
    pub types: HashMap<LookupBuf, String>,
}

inventory::submit! {
    TransformDescription::new::<LogfmtConfig>("logfmt_parser")
}

impl_generate_config_from_default!(LogfmtConfig);

#[async_trait::async_trait]
#[typetag::serde(name = "logfmt_parser")]
impl TransformConfig for LogfmtConfig {
    async fn build(&self) -> crate::Result<Transform> {
        let field = self
            .field
            .clone()
            .unwrap_or_else(|| crate::config::log_schema().message_key().clone());
        let conversions = parse_conversion_map(
            &self
                .types
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )?
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect();

        Ok(Transform::function(Logfmt {
            field,
            drop_field: self.drop_field,
            conversions,
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn transform_type(&self) -> &'static str {
        "logfmt_parser"
    }
}

#[derive(Debug, Clone)]
pub struct Logfmt {
    field: LookupBuf,
    drop_field: bool,
    conversions: HashMap<LookupBuf, Conversion>,
}

impl FunctionTransform for Logfmt {
    fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
        let value = event.as_log().get(&self.field).map(|s| s.to_string_lossy());

        let mut drop_field = self.drop_field;
        if let Some(value) = &value {
            let pairs = logfmt::parse(value)
                .into_iter()
                // Filter out pairs with None value (i.e. non-logfmt data)
                .filter_map(|logfmt::Pair { key, val }| val.map(|val| (key, val)));

            for (key, val) in pairs {
                let key = LookupBuf::from(&*key);
                if key == self.field {
                    drop_field = false;
                }

                if let Some(conv) = self.conversions.get(&key) {
                    match conv.convert::<Value>(val.into()) {
                        Ok(value) => {
                            event.as_mut_log().insert(key, value);
                        }
                        Err(error) => {
                            emit!(LogfmtParserConversionFailed { name: &key, error });
                        }
                    }
                } else {
                    event.as_mut_log().insert(key, val);
                }
            }

            if drop_field {
                event.as_mut_log().remove(&self.field, false);
            }
        } else {
            emit!(LogfmtParserMissingField { field: &self.field });
        };

        emit!(LogfmtParserEventProcessed {});

        output.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::LogfmtConfig;
    use crate::{
        config::TransformConfig,
        event::{LogEvent, Lookup, Value},
        log_event, Event,
    };

    #[test]
    fn generate_config() {
        crate::test_util::test_generate_config::<LogfmtConfig>();
    }

    async fn parse_log(text: &str, drop_field: bool, types: &[(&str, &str)]) -> LogEvent {
        let event = log_event! {
            crate::config::log_schema().message_key().clone() => text.to_string(),
            crate::config::log_schema().message_key().clone() => chrono::Utc::now(),
        };

        let mut parser = LogfmtConfig {
            field: None,
            drop_field,
            types: types.iter().map(|&(k, v)| (k.into(), v.into())).collect(),
        }
        .build()
        .await
        .unwrap();
        let parser = parser.as_function();

        parser.transform_one(event).unwrap().into_log()
    }

    #[tokio::test]
    async fn logfmt_adds_parsed_field_to_event() {
        crate::test_util::trace_init();
        let log = parse_log("status=1234 time=\"5678\"", false, &[]).await;

        assert_eq!(log[Lookup::from("status")], "1234".into());
        assert_eq!(log[Lookup::from("time")], "5678".into());
        assert!(log.get(Lookup::from("message")).is_some());
    }

    #[tokio::test]
    async fn logfmt_does_drop_parsed_field() {
        crate::test_util::trace_init();
        let log = parse_log("status=1234 time=5678", true, &[]).await;

        assert_eq!(log[Lookup::from("status")], "1234".into());
        assert_eq!(log[Lookup::from("time")], "5678".into());
        assert!(log.get(Lookup::from("message")).is_none());
    }

    #[tokio::test]
    async fn logfmt_does_not_drop_same_name_parsed_field() {
        crate::test_util::trace_init();
        let log = parse_log("status=1234 message=yes", true, &[]).await;

        assert_eq!(log[Lookup::from("status")], "1234".into());
        assert_eq!(log[Lookup::from("message")], "yes".into());
    }

    #[tokio::test]
    async fn logfmt_coerces_fields_to_types() {
        crate::test_util::trace_init();
        let log = parse_log(
            "code=1234 flag=yes number=42.3 rest=word",
            false,
            &[("flag", "bool"), ("code", "integer"), ("number", "float")],
        )
        .await;

        assert_eq!(log[Lookup::from("number")], Value::Float(42.3));
        assert_eq!(log[Lookup::from("flag")], Value::Boolean(true));
        assert_eq!(log[Lookup::from("code")], Value::Integer(1234));
        assert_eq!(log[Lookup::from("rest")], Value::Bytes("word".into()));
    }

    #[tokio::test]
    async fn heroku_router_message() {
        crate::test_util::trace_init();
        let log = parse_log(
            r#"at=info method=GET path="/cart_link" host=lumberjack-store.timber.io request_id=05726858-c44e-4f94-9a20-37df73be9006 fwd="73.75.38.87" dyno=web.1 connect=1ms service=22ms status=304 bytes=656 protocol=http"#,
            true,
            &[("status", "integer"), ("bytes", "integer")],
        ).await;

        assert_eq!(log[Lookup::from("at")], "info".into());
        assert_eq!(log[Lookup::from("method")], "GET".into());
        assert_eq!(log[Lookup::from("path")], "/cart_link".into());
        assert_eq!(
            log[Lookup::from("request_id")],
            "05726858-c44e-4f94-9a20-37df73be9006".into(),
        );
        assert_eq!(log[Lookup::from("fwd")], "73.75.38.87".into());
        assert_eq!(log[Lookup::from("dyno")], "web.1".into());
        assert_eq!(log[Lookup::from("connect")], "1ms".into());
        assert_eq!(log[Lookup::from("service")], "22ms".into());
        assert_eq!(log[Lookup::from("status")], Value::Integer(304));
        assert_eq!(log[Lookup::from("bytes")], Value::Integer(656));
        assert_eq!(log[Lookup::from("protocol")], "http".into());
    }

    #[tokio::test]
    async fn logfmt_handles_herokus_weird_octothorpes() {
        crate::test_util::trace_init();
        let log = parse_log("source=web.1 dyno=heroku.2808254.d97d0ea7-cf3d-411b-b453-d2943a50b456 sample#memory_total=21.00MB sample#memory_rss=21.22MB sample#memory_cache=0.00MB sample#memory_swap=0.00MB sample#memory_pgpgin=348836pages sample#memory_pgpgout=343403pages", true, &[]).await;

        assert_eq!(log[Lookup::from("source")], "web.1".into());
        assert_eq!(
            log[Lookup::from("dyno")],
            "heroku.2808254.d97d0ea7-cf3d-411b-b453-d2943a50b456".into()
        );
        assert_eq!(log[Lookup::from("sample#memory_total")], "21.00MB".into());
        assert_eq!(log[Lookup::from("sample#memory_rss")], "21.22MB".into());
        assert_eq!(log[Lookup::from("sample#memory_cache")], "0.00MB".into());
        assert_eq!(log[Lookup::from("sample#memory_swap")], "0.00MB".into());
        assert_eq!(
            log[Lookup::from("sample#memory_pgpgin")],
            "348836pages".into()
        );
        assert_eq!(
            log[Lookup::from("sample#memory_pgpgout")],
            "343403pages".into()
        );
    }
}
