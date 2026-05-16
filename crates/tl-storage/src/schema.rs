diesel::table! {
    agents (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        profile_yaml -> Text,
        parsed_profile -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    policies (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        policy_yaml -> Text,
        parsed_policy -> Jsonb,
        enabled -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
        owner_agent_id -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(agents, policies);

diesel::table! {
    traces (trace_id, created_at) {
        workspace_id -> Text,
        trace_id -> Uuid,
        domain -> Text,
        decision -> Text,
        elapsed_ms -> Int4,
        payload -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    escalations (id) {
        id -> Uuid,
        trace_id -> Uuid,
        webhook_url -> Text,
        status -> Text,
        attempts -> Int4,
        payload -> Jsonb,
        created_at -> Timestamptz,
        sent_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        password_hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
