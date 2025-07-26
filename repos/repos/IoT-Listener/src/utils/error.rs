use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};
use crate::{entities, graph::{PropagationError, RwLockGraph}};

/// Function to upload errors to the database using the sea orm active model
/// Requires the graph to be passed in to get the node ids from the node indexes in the PropagationError
/// Returns the result of the insert operation
pub async fn upload_errors(errors: &[PropagationError], flow_id: &i32, graph: &RwLockGraph, db: &DatabaseConnection) -> Result<sea_orm::InsertResult<entities::flow_error::ActiveModel>, sea_orm::DbErr> {
    // Map the node indexes to node ids
    let graph = graph.read().await;
    let models = errors
        .iter()
        .map(|(node_index, error)| {
            let node_id = graph[*node_index].id().to_string();
            entities::flow_error::ActiveModel {
                flow_id: ActiveValue::set(*flow_id),
                node_id: ActiveValue::set(node_id.clone()),
                message: ActiveValue::set(error.to_string()),
                timestamp: ActiveValue::set(Some(chrono::Utc::now().naive_utc())),
                id: ActiveValue::NotSet,
            }
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        return Ok(sea_orm::InsertResult {
            last_insert_id: 0,
        });
    }
    entities::flow_error::Entity::insert_many(models).exec(db).await
}