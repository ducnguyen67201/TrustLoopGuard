use std::collections::BTreeSet;

use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use tl_core::McpGatewayToolAssignment;
use uuid::Uuid;

use super::McpGatewayRepo;
use crate::models::{NewMcpAgentToolAssignment, NewMcpToolAssignment};
use crate::schema::{
    agents, mcp_agent_tool_assignments, mcp_tool_assignments, mcp_tools, workspace_members,
};
use crate::StorageError;

impl McpGatewayRepo {
    pub async fn replace_agent_assignments(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        agent_id: &str,
        user_ids: Vec<Uuid>,
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, StorageError> {
        let unique = user_ids.into_iter().collect::<BTreeSet<_>>();
        if unique.len() > 500 {
            return Err(StorageError::Conflict);
        }
        let workspace = workspace_id.to_string();
        let agent = agent_id.to_string();
        let values = unique.iter().copied().collect::<Vec<_>>();
        let returned = values.clone();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            let tool_exists = mcp_tools::table
                .filter(mcp_tools::workspace_id.eq(&workspace))
                .filter(mcp_tools::id.eq(tool_id))
                .count()
                .get_result::<i64>(&mut *conn)
                .await?;
            if tool_exists != 1 {
                return Err(StorageError::NotFound);
            }

            let agent_exists = agents::table
                .filter(agents::workspace_id.eq(&workspace))
                .filter(agents::id.eq(&agent))
                .filter(agents::deleted_at.is_null())
                .count()
                .get_result::<i64>(&mut *conn)
                .await?;
            if agent_exists != 1 {
                return Err(StorageError::NotFound);
            }

            if !values.is_empty() {
                let member_count = workspace_members::table
                    .filter(workspace_members::workspace_id.eq(&workspace))
                    .filter(workspace_members::user_id.eq_any(&values))
                    .count()
                    .get_result::<i64>(&mut *conn)
                    .await?;
                if member_count != values.len() as i64 {
                    return Err(StorageError::Conflict);
                }
            }

            let previous = mcp_agent_tool_assignments::table
                .filter(mcp_agent_tool_assignments::workspace_id.eq(&workspace))
                .filter(mcp_agent_tool_assignments::tool_id.eq(tool_id))
                .filter(mcp_agent_tool_assignments::agent_id.eq(&agent))
                .select(mcp_agent_tool_assignments::user_id)
                .load::<Uuid>(&mut *conn)
                .await?;

            diesel::delete(
                mcp_agent_tool_assignments::table
                    .filter(mcp_agent_tool_assignments::workspace_id.eq(&workspace))
                    .filter(mcp_agent_tool_assignments::tool_id.eq(tool_id))
                    .filter(mcp_agent_tool_assignments::agent_id.eq(&agent)),
            )
            .execute(&mut *conn)
            .await?;

            if !values.is_empty() {
                let rows = values
                    .iter()
                    .copied()
                    .map(|user_id| NewMcpAgentToolAssignment {
                        workspace_id: workspace.clone(),
                        tool_id,
                        user_id,
                        agent_id: agent.clone(),
                        created_by,
                    })
                    .collect::<Vec<_>>();
                diesel::insert_into(mcp_agent_tool_assignments::table)
                    .values(rows)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_assignment_error)?;

                let compatibility_rows = values
                    .iter()
                    .copied()
                    .map(|user_id| NewMcpToolAssignment {
                        workspace_id: workspace.clone(),
                        tool_id,
                        user_id,
                        created_by,
                    })
                    .collect::<Vec<_>>();
                diesel::insert_into(mcp_tool_assignments::table)
                    .values(compatibility_rows)
                    .on_conflict_do_nothing()
                    .execute(&mut *conn)
                    .await
                    .map_err(map_assignment_error)?;
            }

            let removed = previous
                .into_iter()
                .filter(|user_id| !unique.contains(user_id))
                .collect::<Vec<_>>();
            for user_id in removed {
                let remaining = mcp_agent_tool_assignments::table
                    .filter(mcp_agent_tool_assignments::workspace_id.eq(&workspace))
                    .filter(mcp_agent_tool_assignments::tool_id.eq(tool_id))
                    .filter(mcp_agent_tool_assignments::user_id.eq(user_id))
                    .count()
                    .get_result::<i64>(&mut *conn)
                    .await?;
                if remaining == 0 {
                    diesel::delete(
                        mcp_tool_assignments::table
                            .filter(mcp_tool_assignments::workspace_id.eq(&workspace))
                            .filter(mcp_tool_assignments::tool_id.eq(tool_id))
                            .filter(mcp_tool_assignments::user_id.eq(user_id)),
                    )
                    .execute(&mut *conn)
                    .await?;
                }
            }
            Ok(returned)
        })
        .await
    }
}

pub(super) async fn assignment_views_for(
    conn: &mut AsyncPgConnection,
    workspace_id: &str,
    tool_id: Uuid,
) -> Result<(Vec<String>, Vec<McpGatewayToolAssignment>, Vec<String>), StorageError> {
    let legacy_ids = mcp_tool_assignments::table
        .filter(mcp_tool_assignments::workspace_id.eq(workspace_id))
        .filter(mcp_tool_assignments::tool_id.eq(tool_id))
        .order(mcp_tool_assignments::user_id.asc())
        .select(mcp_tool_assignments::user_id)
        .load::<Uuid>(conn)
        .await?;
    let pair_rows = mcp_agent_tool_assignments::table
        .filter(mcp_agent_tool_assignments::workspace_id.eq(workspace_id))
        .filter(mcp_agent_tool_assignments::tool_id.eq(tool_id))
        .order((
            mcp_agent_tool_assignments::agent_id.asc(),
            mcp_agent_tool_assignments::user_id.asc(),
        ))
        .select((
            mcp_agent_tool_assignments::user_id,
            mcp_agent_tool_assignments::agent_id,
        ))
        .load::<(Uuid, String)>(conn)
        .await?;
    let bound_users = pair_rows
        .iter()
        .map(|(user_id, _)| *user_id)
        .collect::<BTreeSet<_>>();
    let unbound = legacy_ids
        .iter()
        .filter(|user_id| !bound_users.contains(user_id))
        .map(ToString::to_string)
        .collect();
    let assignments = pair_rows
        .into_iter()
        .map(|(user_id, agent_id)| McpGatewayToolAssignment {
            user_id: user_id.to_string(),
            agent_id,
        })
        .collect();
    Ok((
        legacy_ids.into_iter().map(|id| id.to_string()).collect(),
        assignments,
        unbound,
    ))
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
