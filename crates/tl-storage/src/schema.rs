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
    gateway_routes (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        provider_connection_id -> Text,
        agent_id -> Text,
        reliability_mode -> Text,
        fallback_provider_connection_id -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    notification_rules (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        environment_id -> Text,
        agent_id -> Nullable<Text>,
        email -> Text,
        event_kinds -> Array<Text>,
        enabled -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    notification_deliveries (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        rule_id -> Uuid,
        environment_id -> Text,
        run_id -> Nullable<Uuid>,
        event_kind -> Text,
        subject_id -> Text,
        subject_version -> Text,
        status -> Text,
        payload -> Jsonb,
        attempt_count -> Int4,
        next_attempt_at -> Timestamptz,
        lease_owner -> Nullable<Text>,
        lease_expires_at -> Nullable<Timestamptz>,
        last_error_code -> Nullable<Text>,
        last_error_message -> Nullable<Text>,
        sent_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        agent_id -> Text,
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
        boundary_source -> Nullable<Text>,
        boundary_confidence -> Nullable<Text>,
        finalized_at -> Nullable<Timestamptz>,
        capture_status -> Text,
        capture_deadline -> Nullable<Timestamptz>,
        expected_flush_id -> Nullable<Text>,
        previous_run_id -> Nullable<Uuid>,
        last_evidence_at -> Nullable<Timestamptz>,
        dropped_trace_count -> Int8,
        reevaluation_agent_ids -> Nullable<Array<Text>>,
        evaluation_eligibility -> Text,
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
        agent_id -> Nullable<Text>,
        late_evidence -> Bool,
    }
}

diesel::table! {
    run_participants (workspace_id, run_id, agent_id) {
        workspace_id -> Text,
        environment_id -> Text,
        run_id -> Uuid,
        agent_id -> Text,
        role -> Text,
        joined_at -> Timestamptz,
        manifest_frozen_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    agent_evaluation_profiles (workspace_id, environment_id, agent_id) {
        workspace_id -> Text,
        environment_id -> Text,
        agent_id -> Text,
        enabled -> Bool,
        capture_mode -> Text,
        content_mode -> Text,
        quiet_period_ms -> Int8,
        max_capture_wait_ms -> Int8,
        on_incomplete -> Text,
        profile_version -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    agent_evaluation_policy_assignments (workspace_id, environment_id, agent_id, policy_id) {
        workspace_id -> Text,
        environment_id -> Text,
        agent_id -> Text,
        policy_id -> Text,
        policy_version -> Nullable<Int4>,
        weight -> Int4,
        critical -> Bool,
        enabled -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    run_evaluation_policy_manifest (workspace_id, run_id, agent_id, policy_id) {
        workspace_id -> Text,
        environment_id -> Text,
        run_id -> Uuid,
        agent_id -> Text,
        policy_id -> Text,
        policy_family -> Text,
        policy_version -> Int4,
        policy_hash -> Text,
        policy_yaml -> Text,
        weight -> Int4,
        critical -> Bool,
        evidence_requirements -> Jsonb,
        captured_at -> Timestamptz,
    }
}

diesel::table! {
    run_spans (workspace_id, environment_id, otel_trace_id, otel_span_id) {
        workspace_id -> Text,
        environment_id -> Text,
        run_id -> Uuid,
        agent_id -> Text,
        run_event_id -> Nullable<Uuid>,
        otel_trace_id -> Text,
        otel_span_id -> Text,
        parent_span_id -> Nullable<Text>,
        name -> Text,
        span_kind -> Int4,
        operation_name -> Nullable<Text>,
        conversation_id -> Nullable<Text>,
        external_agent_id -> Nullable<Text>,
        started_at -> Timestamptz,
        ended_at -> Timestamptz,
        status_code -> Int4,
        status_message -> Nullable<Text>,
        resource -> Jsonb,
        attributes -> Jsonb,
        events -> Jsonb,
        links -> Jsonb,
        content_capture_status -> Text,
        dropped_attribute_count -> Int4,
        late_evidence -> Bool,
        ingested_at -> Timestamptz,
    }
}

diesel::table! {
    otel_flush_receipts (workspace_id, environment_id, run_id, flush_id) {
        workspace_id -> Text,
        environment_id -> Text,
        run_id -> Uuid,
        flush_id -> Text,
        accepted_span_count -> Int4,
        rejected_span_count -> Int4,
        accepted_at -> Timestamptz,
    }
}

diesel::table! {
    run_snapshots (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        run_id -> Uuid,
        snapshot_version -> Int4,
        snapshot_hash -> Text,
        manifest_hash -> Text,
        capture_status -> Text,
        event_cutoff -> Timestamptz,
        event_count -> Int8,
        trace_count -> Int8,
        span_count -> Int8,
        dropped_trace_count -> Int8,
        late_evidence_count -> Int8,
        snapshot -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_jobs (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        run_id -> Uuid,
        agent_id -> Text,
        snapshot_id -> Uuid,
        snapshot_hash -> Text,
        manifest_hash -> Text,
        evaluator_version -> Text,
        status -> Text,
        attempts -> Int4,
        available_at -> Timestamptz,
        lease_owner -> Nullable<Text>,
        lease_expires_at -> Nullable<Timestamptz>,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_results (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        job_id -> Uuid,
        run_id -> Uuid,
        agent_id -> Text,
        snapshot_hash -> Text,
        manifest_hash -> Text,
        evaluator_version -> Text,
        verdict -> Text,
        score_bps -> Nullable<Int4>,
        capture_status -> Text,
        llm_audit -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_findings (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        result_id -> Uuid,
        run_id -> Uuid,
        agent_id -> Text,
        policy_id -> Text,
        policy_version -> Int4,
        severity -> Text,
        critical -> Bool,
        status -> Text,
        score_bps -> Nullable<Int4>,
        reason -> Text,
        evidence -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_datasets (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        agent_id -> Text,
        name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_dataset_versions (workspace_id, dataset_id, version) {
        workspace_id -> Text,
        dataset_id -> Uuid,
        version -> Int4,
        manifest_hash -> Text,
        manifest -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_cases (workspace_id, dataset_id, dataset_version, case_id) {
        workspace_id -> Text,
        dataset_id -> Uuid,
        dataset_version -> Int4,
        case_id -> Text,
        case_hash -> Text,
        scoring_mode -> Text,
        weight -> Int4,
        spec -> Jsonb,
    }
}

diesel::table! {
    evaluation_campaigns (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        dataset_id -> Uuid,
        dataset_version -> Int4,
        agent_id -> Text,
        status -> Text,
        case_runs -> Jsonb,
        aggregate -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    evaluation_release_gates (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        agent_id -> Text,
        campaign_id -> Nullable<Uuid>,
        manifest_hash -> Text,
        verdict -> Text,
        evidence -> Jsonb,
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
        is_approved -> Bool,
        is_platform_admin -> Bool,
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
        is_knowledge_base_enabled -> Bool,
        is_attacks_enabled -> Bool,
        is_mcp_gateway_enabled -> Bool,
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
    github_installation_states (state_hash) {
        state_hash -> Bytea,
        workspace_id -> Text,
        user_id -> Uuid,
        expires_at -> Timestamptz,
        consumed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    github_installations (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        installation_id -> Int8,
        account_login -> Text,
        account_type -> Text,
        repository_selection -> Text,
        status -> Text,
        installed_by_user_id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    github_repository_connections (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        installation_id -> Uuid,
        repository_id -> Int8,
        owner -> Text,
        name -> Text,
        default_branch -> Text,
        root_path -> Text,
        agent_id -> Text,
        environment_id -> Text,
        status -> Text,
        recipe_version -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    github_integration_jobs (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        connection_id -> Uuid,
        status -> Text,
        risk_statement -> Text,
        base_branch -> Text,
        base_sha -> Nullable<Text>,
        recipe_version -> Text,
        analysis_summary -> Nullable<Jsonb>,
        proposed_changes -> Jsonb,
        manual_steps -> Jsonb,
        branch_name -> Nullable<Text>,
        commit_sha -> Nullable<Text>,
        pull_request_number -> Nullable<Int8>,
        pull_request_url -> Nullable<Text>,
        error_code -> Nullable<Text>,
        error_message -> Nullable<Text>,
        attempt_count -> Int4,
        installation_connected_at -> Nullable<Timestamptz>,
        repository_connected_at -> Nullable<Timestamptz>,
        analysis_completed_at -> Nullable<Timestamptz>,
        pr_opened_at -> Nullable<Timestamptz>,
        pr_merged_at -> Nullable<Timestamptz>,
        first_verified_trace_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        environment_id -> Text,
        id -> Uuid,
        idempotency_key -> Text,
        principal_id -> Text,
        action_kind -> Text,
        operation -> Text,
        amount_minor -> Int8,
        currency -> Text,
        counterparty -> Nullable<Jsonb>,
        rail -> Text,
        memo -> Nullable<Text>,
        metadata -> Jsonb,
        evidence -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        authorization_intent_id -> Nullable<Uuid>,
        execution_status -> Text,
    }
}

diesel::table! {
    authorization_intents (workspace_id, environment_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        domain -> Text,
        subject_id -> Text,
        idempotency_key -> Text,
        principal_id -> Text,
        operation -> Text,
        fingerprint -> Text,
        fingerprint_version -> Int4,
        subject_snapshot -> Jsonb,
        status -> Text,
        current_effect -> Text,
        reason -> Text,
        trace_id -> Nullable<Text>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    authorization_approvals (workspace_id, environment_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        intent_id -> Uuid,
        fingerprint -> Text,
        status -> Text,
        envelope -> Jsonb,
        envelope_hash -> Text,
        requirement_ids -> Jsonb,
        approver_roles -> Jsonb,
        decided_by -> Nullable<Text>,
        decided_at -> Nullable<Timestamptz>,
        decision_reason -> Nullable<Text>,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    authorization_grants (workspace_id, environment_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        principal_id -> Text,
        domain -> Text,
        capability -> Text,
        mode -> Text,
        status -> Text,
        source -> Text,
        scope_schema -> Text,
        scope -> Nullable<Jsonb>,
        exact_fingerprint -> Nullable<Text>,
        fingerprint_version -> Int4,
        source_approval_id -> Nullable<Uuid>,
        requirement_ids -> Jsonb,
        max_uses -> Nullable<Int4>,
        use_count -> Int4,
        starts_at -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        revoked_at -> Nullable<Timestamptz>,
        revoked_by -> Nullable<Text>,
        created_by -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    authorization_leases (workspace_id, environment_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        intent_id -> Uuid,
        grant_id -> Nullable<Uuid>,
        attempt_id -> Text,
        fingerprint -> Text,
        status -> Text,
        claimed_at -> Timestamptz,
        consumed_at -> Nullable<Timestamptz>,
        canceled_at -> Nullable<Timestamptz>,
        expires_at -> Timestamptz,
        outcome -> Jsonb,
    }
}

diesel::table! {
    authorization_receipts (workspace_id, environment_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        intent_id -> Nullable<Uuid>,
        trace_id -> Nullable<Text>,
        principal_id -> Nullable<Text>,
        operation -> Nullable<Text>,
        run_id -> Nullable<Uuid>,
        domain -> Text,
        effect -> Text,
        intent_status -> Nullable<Text>,
        subject_hash -> Text,
        reason -> Text,
        findings -> Jsonb,
        policy_versions -> Jsonb,
        approval_id -> Nullable<Uuid>,
        grant_id -> Nullable<Uuid>,
        lease_id -> Nullable<Uuid>,
        domain_evidence -> Jsonb,
        created_at -> Timestamptz,
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
    financial_budget_principal_locks (workspace_id, principal_id, currency) {
        workspace_id -> Text,
        principal_id -> Text,
        currency -> Text,
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
        meter -> Text,
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
        meter -> Text,
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
    llm_budget_principal_locks (workspace_id, principal_id) {
        workspace_id -> Text,
        principal_id -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    llm_budget_reservations (workspace_id, request_id) {
        workspace_id -> Text,
        request_id -> Text,
        principal_id -> Text,
        api_key_id -> Text,
        currency -> Text,
        reserved_nanos -> Int8,
        actual_nanos -> Nullable<Int8>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    llm_model_prices (workspace_id, model) {
        workspace_id -> Text,
        model -> Text,
        input_per_million_minor -> Int8,
        output_per_million_minor -> Int8,
        input_per_million_nanos -> Int8,
        output_per_million_nanos -> Int8,
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
        usage_kind -> Text,
        model -> Text,
        prompt_tokens -> Int8,
        completion_tokens -> Int8,
        cost_minor -> Int8,
        cost_nanos -> Int8,
        currency -> Text,
        request_id -> Text,
        metadata -> Jsonb,
        effective_at -> Timestamptz,
    }
}

diesel::table! {
    financial_receipts (workspace_id, id) {
        workspace_id -> Text,
        environment_id -> Text,
        id -> Uuid,
        action_id -> Uuid,
        authorization_receipt_id -> Nullable<Uuid>,
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

diesel::table! {
    mcp_oauth_clients (client_id) {
        client_id -> Text,
        client_name -> Nullable<Text>,
        redirect_uris -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_oauth_authorization_codes (code_hash) {
        code_hash -> Text,
        client_id -> Text,
        redirect_uri -> Text,
        user_id -> Uuid,
        username -> Text,
        workspace_id -> Text,
        agent_id -> Nullable<Text>,
        resource -> Text,
        scope -> Text,
        code_challenge -> Text,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_oauth_refresh_tokens (token_hash) {
        token_hash -> Text,
        client_id -> Text,
        user_id -> Uuid,
        username -> Text,
        workspace_id -> Text,
        agent_id -> Nullable<Text>,
        resource -> Text,
        scope -> Text,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_server_connections (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        display_name -> Text,
        server_slug -> Text,
        endpoint_url -> Text,
        auth_kind -> Text,
        encrypted_credential -> Nullable<Text>,
        enabled -> Bool,
        last_sync_status -> Text,
        last_sync_error -> Nullable<Text>,
        last_synced_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_tools (workspace_id, id) {
        workspace_id -> Text,
        id -> Uuid,
        connection_id -> Uuid,
        upstream_name -> Text,
        public_name -> Text,
        title -> Nullable<Text>,
        description -> Nullable<Text>,
        input_schema -> Jsonb,
        output_schema -> Nullable<Jsonb>,
        annotations -> Jsonb,
        schema_hash -> Text,
        side_effect -> Text,
        catalog_status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_agent_tool_assignments (workspace_id, tool_id, user_id, agent_id) {
        workspace_id -> Text,
        tool_id -> Uuid,
        user_id -> Uuid,
        agent_id -> Text,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    mcp_tool_assignments (workspace_id, tool_id, user_id) {
        workspace_id -> Text,
        tool_id -> Uuid,
        user_id -> Uuid,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
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
diesel::joinable!(gateway_routes -> workspaces (workspace_id));
diesel::joinable!(notification_rules -> workspaces (workspace_id));
diesel::joinable!(notification_deliveries -> workspaces (workspace_id));
diesel::joinable!(analytics_dashboard_views -> workspaces (workspace_id));
diesel::joinable!(financial_actions -> workspaces (workspace_id));
diesel::joinable!(financial_action_outcomes -> workspaces (workspace_id));
diesel::joinable!(financial_payment_sessions -> workspaces (workspace_id));
diesel::joinable!(counterparties -> workspaces (workspace_id));

diesel::allow_tables_to_appear_in_same_query!(
    analytics_dashboard_views,
    agents,
    github_installation_states,
    github_installations,
    github_repository_connections,
    github_integration_jobs,
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
    gateway_routes,
    notification_rules,
    notification_deliveries,
    human_review_events,
    run_events,
    runs,
    run_participants,
    agent_evaluation_profiles,
    agent_evaluation_policy_assignments,
    run_evaluation_policy_manifest,
    run_spans,
    otel_flush_receipts,
    run_snapshots,
    evaluation_jobs,
    evaluation_results,
    evaluation_findings,
    evaluation_datasets,
    evaluation_dataset_versions,
    evaluation_cases,
    evaluation_campaigns,
    evaluation_release_gates,
    redteam_jobs,
    redteam_attack_sessions,
    redteam_session_events,
    redteam_plans,
    redteam_report_shares,
    financial_actions,
    financial_action_events,
    financial_ledger_entries,
    financial_budget_principal_locks,
    financial_payment_sessions,
    financial_payment_reservations,
    budget_alert_configs,
    budget_alert_firings,
    llm_budget_principal_locks,
    llm_budget_reservations,
    llm_model_prices,
    llm_usage_events,
    financial_receipts,
    financial_action_outcomes,
    counterparties,
    authorization_intents,
    authorization_approvals,
    authorization_grants,
    authorization_leases,
    authorization_receipts,
    mcp_oauth_clients,
    mcp_oauth_authorization_codes,
    mcp_oauth_refresh_tokens,
    mcp_server_connections,
    mcp_tools,
    mcp_agent_tool_assignments,
    mcp_tool_assignments,
);
