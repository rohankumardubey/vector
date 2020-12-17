use super::InternalEvent;
use metrics::counter;
use shared::event::metric::{MetricKind, MetricValue};

#[derive(Debug)]
pub struct StatsdInvalidMetricReceived<'a> {
    pub value: &'a MetricValue,
    pub kind: &'a MetricKind,
}

impl<'a> InternalEvent for StatsdInvalidMetricReceived<'a> {
    fn emit_logs(&self) {
        warn!(
            message = "Invalid metric received; dropping event.",
            value = ?self.value,
            kind = ?self.kind,
            rate_limit_secs = 30,
        )
    }

    fn emit_metrics(&self) {
        counter!("processing_errors_total", 1, "error_type" => "invalid_metric");
    }
}
