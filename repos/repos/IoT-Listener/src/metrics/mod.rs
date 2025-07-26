use once_cell::sync::Lazy;
use rocket_prometheus::{ prometheus::{GaugeVec, IntCounterVec, Opts}, PrometheusMetrics };

pub static SOURCE_MESSAGE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        Opts::new(
            "source_message_count",
            "Number of messages received from sources",
        ),
        &["source"],
    )
    .expect("Failed to create source_message_count counter")
});

pub static SOURCE_ERROR_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        Opts::new(
            "source_error_count",
            "Number of errors received from sources",
        ),
        &["source"],
    )
    .expect("Failed to create source_error_count counter")
});

pub static SOURCE_LAST_ERROR_TIME: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        Opts::new(
            "source_last_error_time",
            "Time of the last error received from sources",
        ),
        &["source"],
    )
    .expect("Failed to create source_last_error_time gauge")
});

pub static READINGS_SAVED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        Opts::new(
            "readings_saved_total",
            "Number of readings saved by the listener"
        ),
        &["flow_id"]
    )
    .expect("Failed to create readings_saved_total counter")
});

#[launch]
pub fn rocket() -> _ {
    let prometheus = PrometheusMetrics::new();
    prometheus.registry().register(Box::new(SOURCE_MESSAGE_COUNT.clone()))
        .expect("Failed to register source_message_count counter");
    prometheus.registry().register(Box::new(SOURCE_ERROR_COUNT.clone()))
        .expect("Failed to register source_error_count counter");
    prometheus.registry().register(Box::new(SOURCE_LAST_ERROR_TIME.clone()))
        .expect("Failed to register source_last_error_time");
    prometheus.registry().register(Box::new(READINGS_SAVED_TOTAL.clone()))
        .expect("Failed to register readings_saved_total");

    rocket
        ::build()
        .mount("/metrics", prometheus)
}
