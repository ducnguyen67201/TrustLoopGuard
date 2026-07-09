diesel::table! {
    workspace_environments (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        slug -> Text,
        name -> Text,
        description -> Nullable<Text>,
        is_default -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    policy_environment_deployments (workspace_id, environment_id, policy_id) {
        workspace_id -> Text,
        environment_id -> Text,
        policy_id -> Text,
        enabled -> Bool,
        deployed_version -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    analytics_dashboard_views (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        name -> Text,
        is_default -> Bool,
        config -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gateway_provider_connections (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        kind -> Text,
        base_url -> Nullable<Text>,
        default_model -> Text,
        encrypted_api_key -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    enforcement_profiles (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        input_action -> Text,
        output_action -> Text,
        fail_mode -> Text,
        retention_mode -> Text,
        response_mode -> Text,
        fallback_message -> Text,
        max_regenerations -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gateway_routes (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        provider_connection_id -> Text,
        agent_id -> Text,
        enforcement_profile_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

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
        family -> Nullable<Text>,
    }
}

diesel::table! {
    human_review_events (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        trace_id -> Uuid,
        run_id -> Nullable<Uuid>,
        run_event_id -> Nullable<Uuid>,
        outcome -> Text,
        reviewer_id -> Nullable<Text>,
        reason_codes -> Jsonb,
        note -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    run_events (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        run_id -> Uuid,
        sequence -> Int4,
        kind -> Text,
        label -> Nullable<Text>,
        input_summary -> Nullable<Text>,
        output_summary -> Nullable<Text>,
        metadata -> Jsonb,
        occurred_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    runs (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        environment_id -> Text,
        agent_id -> Text,
        kind -> Text,
        status -> Text,
        external_id -> Nullable<Text>,
        metadata -> Jsonb,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    traces (trace_id, created_at) {
        workspace_id -> Text,
        trace_id -> Uuid,
        run_id -> Nullable<Uuid>,
        environment_id -> Text,
        domain -> Text,
        decision -> Text,
        elapsed_ms -> Int4,
        payload -> Jsonb,
        created_at -> Timestamptz,
        run_event_id -> Nullable<Uuid>,
        session_id -> Nullable<Text>,
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
        is_approved -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_identities (provider, provider_subject) {
        provider -> Text,
        provider_subject -> Text,
        user_id -> Uuid,
        email -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    organizations (id) {
        id -> Text,
        name -> Text,
        slug -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    organization_members (organization_id, user_id) {
        organization_id -> Text,
        user_id -> Uuid,
        role -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    workspaces (id) {
        id -> Text,
        organization_id -> Text,
        name -> Text,
        slug -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    workspace_members (workspace_id, user_id) {
        workspace_id -> Text,
        user_id -> Uuid,
        role -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    workspace_invites (id) {
        id -> Text,
        workspace_id -> Text,
        email -> Text,
        role -> Text,
        status -> Text,
        invited_by_user_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    workspace_settings (workspace_id) {
        workspace_id -> Text,
        default_action -> Text,
        escalation_webhook_url -> Nullable<Text>,
        telemetry_enabled -> Bool,
        retention_days -> Text,
        config -> Jsonb,
        updated_at -> Timestamptz,
        data_handling_mode -> Text,
        flow_checker_mode -> Text,
        memory_checker_mode -> Text,
        param_checker_mode -> Text,
        approval_checker_mode -> Text,
    }
}

diesel::table! {
    environment_checker_modes (workspace_id, environment_id) {
        workspace_id -> Text,
        environment_id -> Text,
        flow_checker_mode -> Nullable<Text>,
        memory_checker_mode -> Nullable<Text>,
        param_checker_mode -> Nullable<Text>,
        approval_checker_mode -> Nullable<Text>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workspace_api_keys (id) {
        id -> Text,
        workspace_id -> Text,
        environment_id -> Text,
        name -> Text,
        key_prefix -> Text,
        key_hash -> Text,
        status -> Text,
        created_by_user_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        last_used_at -> Nullable<Timestamptz>,
        revoked_at -> Nullable<Timestamptz>,
        principal_id -> Nullable<Text>,
    }
}

diesel::table! {
    knowledge_sources (id) {
        id -> Text,
        workspace_id -> Text,
        title -> Text,
        kind -> Text,
        location -> Nullable<Text>,
        status -> Text,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_indexed_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    knowledge_source_files (knowledge_source_id) {
        knowledge_source_id -> Text,
        file_name -> Text,
        media_type -> Text,
        byte_size -> Int4,
        checksum_sha256 -> Text,
        data -> Binary,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    tool_metadata (workspace_id, tool) {
        workspace_id -> Text,
        tool -> Text,
        side_effect -> Text,
        reversible -> Bool,
        spec -> Jsonb,
        enabled -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    entity_versions (workspace_id, entity_type, entity_id, version) {
        workspace_id -> Text,
        entity_type  -> Text,
        entity_id    -> Text,
        version      -> Int4,
        content      -> Text,
        created_at   -> Timestamptz,
    }
}

diesel::table! {
    redteam_jobs (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        environment_id -> Text,
        status -> Text,
        target -> Text,
        profile -> Text,
        generator -> Text,
        agent_id -> Nullable<Text>,
        attacks -> Int8,
        landed -> Int8,
        blocked -> Int8,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    redteam_plans (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        environment_id -> Text,
        agent_id -> Text,
        name -> Text,
        plan -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    redteam_attack_sessions (workspace_id, job_id, session_id) {
        workspace_id -> Text,
        job_id -> Uuid,
        session_id -> Text,
        runner_session_id -> Nullable<Text>,
        seq -> Int4,
        case_id -> Nullable<Text>,
        track -> Nullable<Text>,
        kind -> Nullable<Text>,
        trial_index -> Nullable<Int4>,
        attack -> Text,
        goal -> Text,
        status -> Text,
        outcome -> Text,
        landed -> Bool,
        trace_id -> Nullable<Text>,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    redteam_session_events (workspace_id, job_id, session_id, event_id) {
        workspace_id -> Text,
        job_id -> Uuid,
        session_id -> Text,
        event_id -> Text,
        seq -> Int4,
        kind -> Text,
        actor -> Text,
        label -> Nullable<Text>,
        content_text -> Nullable<Text>,
        payload -> Jsonb,
        trace_id -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    financial_actions (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        idempotency_key -> Text,
        principal_id -> Text,
        action_kind -> Text,
        operation -> Text,
        status -> Text,
        amount_minor -> Int8,
        currency -> Text,
        counterparty -> Nullable<Jsonb>,
        mandate -> Nullable<Jsonb>,
        rail -> Text,
        memo -> Nullable<Text>,
        metadata -> Jsonb,
        evidence -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    financial_action_events (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        event_type -> Text,
        from_status -> Nullable<Text>,
        to_status -> Nullable<Text>,
        actor_id -> Nullable<Text>,
        reason -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    financial_ledger_entries (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        entry_kind -> Text,
        amount_minor -> Int8,
        currency -> Text,
        idempotency_key -> Text,
        metadata -> Jsonb,
        effective_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    financial_payment_sessions (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        principal_id -> Text,
        currency -> Text,
        max_amount_minor -> Int8,
        reserved_minor -> Int8,
        committed_minor -> Int8,
        released_minor -> Int8,
        status -> Text,
        expires_at -> Timestamptz,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    financial_payment_reservations (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        session_id -> Text,
        principal_id -> Text,
        payment_requirement_hash -> Text,
        amount_minor -> Int8,
        currency -> Text,
        status -> Text,
        expires_at -> Timestamptz,
        commit_proof -> Nullable<Jsonb>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        committed_at -> Nullable<Timestamptz>,
        released_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    budget_alert_configs (id) {
        id -> Uuid,
        workspace_id -> Text,
        name -> Text,
        window -> Text,
        principal_id -> Nullable<Text>,
        threshold_type -> Text,
        threshold_value -> Int8,
        webhook_url -> Nullable<Text>,
        enabled -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    budget_alert_firings (id) {
        id -> Uuid,
        workspace_id -> Text,
        config_id -> Uuid,
        principal_id -> Text,
        window_start -> Timestamptz,
        cap_minor -> Int8,
        spent_minor -> Int8,
        currency -> Text,
        payload -> Jsonb,
        fired_at -> Timestamptz,
    }
}

diesel::table! {
    llm_model_prices (workspace_id, model) {
        workspace_id -> Text,
        model -> Text,
        input_per_million_minor -> Int8,
        output_per_million_minor -> Int8,
        currency -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    llm_usage_events (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        principal_id -> Text,
        api_key_id -> Text,
        model -> Text,
        prompt_tokens -> Int8,
        completion_tokens -> Int8,
        cost_minor -> Int8,
        currency -> Text,
        request_id -> Text,
        metadata -> Jsonb,
        effective_at -> Timestamptz,
    }
}

diesel::table! {
    mandates (workspace_id, id, version) {
        workspace_id -> Text,
        id -> Text,
        version -> Int4,
        status -> Text,
        principal_id -> Text,
        scope -> Jsonb,
        metadata -> Jsonb,
        starts_at -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    approval_requests (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        status -> Text,
        reason -> Text,
        approver_roles -> Jsonb,
        decided_by -> Nullable<Text>,
        decided_at -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    financial_receipts (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        trace_id -> Nullable<Uuid>,
        ledger_event_ids -> Jsonb,
        proof -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    financial_action_outcomes (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        status -> Text,
        reversal_capability -> Text,
        recovery_status -> Text,
        provider_status -> Nullable<Text>,
        provider_reference -> Nullable<Text>,
        final_loss_amount_minor -> Nullable<Int8>,
        final_loss_currency -> Nullable<Text>,
        occurred_at -> Timestamptz,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    counterparties (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        kind -> Text,
        display_name -> Nullable<Text>,
        country -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    redteam_report_shares (token) {
        token -> Text,
        workspace_id -> Text,
        job_id -> Uuid,
        compare_job_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
        revoked_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(organization_members -> organizations (organization_id));
diesel::joinable!(organization_members -> users (user_id));
diesel::joinable!(oauth_identities -> users (user_id));
diesel::joinable!(workspaces -> organizations (organization_id));
diesel::joinable!(workspace_members -> users (user_id));
diesel::joinable!(workspace_members -> workspaces (workspace_id));
diesel::joinable!(workspace_invites -> users (invited_by_user_id));
diesel::joinable!(workspace_invites -> workspaces (workspace_id));
diesel::joinable!(workspace_settings -> workspaces (workspace_id));
diesel::joinable!(workspace_api_keys -> users (created_by_user_id));
diesel::joinable!(workspace_api_keys -> workspaces (workspace_id));
diesel::joinable!(workspace_environments -> workspaces (workspace_id));
diesel::joinable!(knowledge_source_files -> knowledge_sources (knowledge_source_id));
diesel::joinable!(gateway_provider_connections -> workspaces (workspace_id));
diesel::joinable!(enforcement_profiles -> workspaces (workspace_id));
diesel::joinable!(gateway_routes -> workspaces (workspace_id));
diesel::joinable!(analytics_dashboard_views -> workspaces (workspace_id));
diesel::joinable!(financial_actions -> workspaces (workspace_id));
diesel::joinable!(financial_action_outcomes -> workspaces (workspace_id));
diesel::joinable!(financial_payment_sessions -> workspaces (workspace_id));
diesel::joinable!(mandates -> workspaces (workspace_id));
diesel::joinable!(counterparties -> workspaces (workspace_id));

diesel::allow_tables_to_appear_in_same_query!(
    analytics_dashboard_views,
    agents,
    workspace_environments,
    policy_environment_deployments,
    policies,
    entity_versions,
    traces,
    escalations,
    users,
    oauth_identities,
    organizations,
    organization_members,
    workspaces,
    workspace_members,
    workspace_invites,
    workspace_settings,
    environment_checker_modes,
    workspace_api_keys,
    knowledge_sources,
    knowledge_source_files,
    gateway_provider_connections,
    enforcement_profiles,
    gateway_routes,
    human_review_events,
    run_events,
    runs,
    redteam_jobs,
    redteam_attack_sessions,
    redteam_session_events,
    redteam_plans,
    redteam_report_shares,
    financial_actions,
    financial_action_events,
    financial_ledger_entries,
    financial_payment_sessions,
    financial_payment_reservations,
    budget_alert_configs,
    budget_alert_firings,
    llm_model_prices,
    llm_usage_events,
    mandates,
    approval_requests,
    financial_receipts,
    financial_action_outcomes,
    counterparties,
);
