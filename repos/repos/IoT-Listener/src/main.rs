#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[cfg_attr(coverage_nightly, coverage(off))]
mod entities;
mod graph;
#[cfg_attr(coverage_nightly, coverage(off))]
mod metrics;
mod nodes;
mod readings;
pub mod sources;
mod utils;

#[macro_use]
extern crate rocket;
extern crate paho_mqtt as mqtt;
use graph::build_graph::read_graph;
use metrics::rocket;
use readings::StoreReading;
use sea_orm::{Database, DatabaseConnection, DbErr};
use std::{env, sync::Arc};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::{self, EnvFilter};
use utils::{error::upload_errors, subscribe::subscribe_to_sources};

/// Get the database connection
async fn get_database_connection() -> Result<DatabaseConnection, DbErr> {
    let database_url: String = env::var("uri").expect("uri must be set");
    Database::connect(&database_url).await
}

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the logger without logging sqlx::query calls
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap()); // Ensure info level logs are enabled

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Load the .env file
    dotenv::dotenv().ok();

    // Build the graph from the env file
    let (flow_id, mut graph) = match read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to read flow: {}", e);
            return Err("Failed to read flow".into());
        }
    };
    info!("Loaded flow {:?}", graph);

    // Connect to database
    let conn = match get_database_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err("Failed to connect to database".into());
        }
    };
    info!("Connected to database");
    // Wrap the connection in an Arc to share it between threads
    let db = Arc::new(conn);

    let subscription_db = Arc::clone(&db);

    let global_cancellation_token = CancellationToken::new();
    let (mut source_monitors_join_handles, mut sources_incoming_data_pipe_receiver) =
        subscribe_to_sources(
            &*graph.read().await,
            &global_cancellation_token,
            subscription_db,
        )?;

    let receiver_cancel = CancellationToken::clone(&global_cancellation_token);
    let store_db = Arc::clone(&db);
    source_monitors_join_handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = receiver_cancel.cancelled() => {
                    info!("Cancellation token triggered, shutting down");
                    break;
                },
                Some(trigger) = sources_incoming_data_pipe_receiver.recv() => {
                    info!("Received trigger: {:?}", trigger);

                    let (readings, _, node_errors) = graph.backpropagate(trigger).await;
                    // info!("readings: {:?}", readings);

                    info!("--- DEBUG START ---");
                    info!("Readings: {:#?}", readings);
                    info!("Node Errors: {:#?}", node_errors);
                    info!("--- DEBUG END ---");


                    let mut reading_errors = Vec::new();
                    for reading in readings {
                        reading.store(&store_db, flow_id).await.unwrap_or_else(|e| {
                            error!("Failed to store reading: {}; {:?}", e, reading);
                            reading_errors.push(e);
                        });
                    }
                    let res = upload_errors(&node_errors, &flow_id, &graph, &store_db).await;
                    if let Err(e) = res {
                        error!("Failed to upload errors: {}", e);
                    }
                }
            }
        }
    }));

    // Start the rocket server
    let rocket_handle = tokio::spawn(async move { rocket().launch().await.ok() });

    // Add the rocket handle to the join handles
    let rocket_cancel = CancellationToken::clone(&global_cancellation_token);

    // Spawn a thread to wait for the rocket server to finish or cancellation
    source_monitors_join_handles.push(tokio::spawn(async move {
        tokio::select! {
            _ = rocket_cancel.cancelled() => {
                info!("Cancellation token triggered, shutting down");
            },
            res = rocket_handle => {
                if let Err(e) = res {
                    error!("Failed to start Rocket: {}", e);
                }
            }
        }
    }));

    match signal::ctrl_c().await {
        Ok(_) => info!("Received Ctrl-C, shutting down"),
        Err(e) => {
            error!("Error receiving Ctrl-C: {:?}", e);
            return Err(format!("Error receiving Ctrl-C: {:?}", e).into());
        }
    }

    global_cancellation_token.cancel();

    // Wait for all threads to finish
    for handle in source_monitors_join_handles {
        handle.await?;
    }

    Arc::try_unwrap(db)
        .expect("Failed to unwrap Arc")
        .close()
        .await?;
    info!("Database connection closed");

    Ok(())
}