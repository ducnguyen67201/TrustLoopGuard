use std::collections::BTreeSet;

use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use super::McpGatewayRepo;
use crate::models::NewMcpToolAssignment;
use crate::schema::{mcp_tool_assignments, mcp_tools};
use crate::StorageError;

impl McpGatewayRepo {
    pub async fn replace_assignments(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        user_ids: Vec<Uuid>,
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, StorageError> {
        let unique = user_ids.into_iter().collect::<BTreeSet<_>>();
        if unique.len() > 500 {
            return Err(StorageError::Conflict);
        }
        let workspace = workspace_id.to_string();
        let values = unique.iter().copied().collect::<Vec<_>>();
        let returned = values.clone();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            let exists = mcp_tools::table
                .filter(mcp_tools::workspace_id.eq(&workspace))
                .filter(mcp_tools::id.eq(tool_id))
                .count()
                .get_result::<i64>(&mut *conn)
                .await?;
            if exists != 1 {
                return Err(StorageError::NotFound);
            }

            diesel::delete(
                mcp_tool_assignments::table
                    .filter(mcp_tool_assignments::workspace_id.eq(&workspace))
                    .filter(mcp_tool_assignments::tool_id.eq(tool_id)),
            )
            .execute(&mut *conn)
            .await?;

            if !values.is_empty() {
                let rows = values
                    .into_iter()
                    .map(|user_id| NewMcpToolAssignment {
                        workspace_id: workspace.clone(),
                        tool_id,
                        user_id,
                        created_by,
                    })
                    .collect::<Vec<_>>();
                diesel::insert_into(mcp_tool_assignments::table)
                    .values(rows)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_assignment_error)?;
            }
            Ok(returned)
        })
        .await
    }
}

pub(super) async fn assigned_user_ids_for(
    conn: &mut AsyncPgConnection,
    workspace_id: &str,
    tool_id: Uuid,
) -> Result<Vec<String>, StorageError> {
    let ids = mcp_tool_assignments::table
        .filter(mcp_tool_assignments::workspace_id.eq(workspace_id))
        .filter(mcp_tool_assignments::tool_id.eq(tool_id))
        .order(mcp_tool_assignments::user_id.asc())
        .select(mcp_tool_assignments::user_id)
        .load::<Uuid>(conn)
        .await?;
    Ok(ids.into_iter().map(|id| id.to_string()).collect())
}

fn map_assignment_error(error: diesel::result::Error) -> StorageError {
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            _,
        ) => StorageError::Conflict,
        other => other.into(),
    }
}
