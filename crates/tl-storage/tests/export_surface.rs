use tl_storage::{DecisionStore, MemoryStore, StorageError};

#[test]
fn default_storage_exports_remain_available() {
    fn accepts_store<T: DecisionStore>(_store: &T) {}

    let store = MemoryStore::default();
    accepts_store(&store);

    let error = StorageError::NotFound;
    assert_eq!(error.to_string(), "not found");
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_storage_exports_remain_available() {
    use tl_storage::{
        connect_postgres, migrate_postgres, schema, spawn_writer, AddMemberOutcome, AgentRepo,
        AnalyticsRepo, DashboardAdminRepo, EnforcementProfilePatch, EscalationRepo, EscalationRow,
        FinancialLedgerEntryKind, FinancialRepo, GatewayProviderConnectionSecret, GatewayRepo,
        GatewayRoutePatch, HumanReviewAnalyticsFilter, HumanReviewRepo, KnowledgeFileRow,
        KnowledgeRepo, KnowledgeSourceRow, NewKnowledgeFile, NewKnowledgeSource, PolicyRepo,
        PolicyRow, PostgresStore, ResolvedGatewayRoute, RunFilter, RunRepo, SourceLabelPolicyRepo,
        StoredFinancialAction, StoredFinancialActionEvent, StoredFinancialActionOutcome,
        StoredFinancialApprovalRequest, StoredFinancialMandate, StoredFinancialReceipt,
        StoredSourceLabelPolicy, StoredToolMetadata, TeamRepo, ToolMetadataRepo, TraceRepo,
        TraceRow, TraceWrite, UserRecord, UserRepo, WriterConfig,
    };

    fn assert_type<T>() {}

    assert_type::<AgentRepo>();
    assert_type::<AnalyticsRepo>();
    assert_type::<DashboardAdminRepo>();
    assert_type::<EnforcementProfilePatch>();
    assert_type::<EscalationRepo>();
    assert_type::<EscalationRow>();
    assert_type::<FinancialLedgerEntryKind>();
    assert_type::<FinancialRepo>();
    assert_type::<GatewayProviderConnectionSecret>();
    assert_type::<GatewayRepo>();
    assert_type::<GatewayRoutePatch>();
    assert_type::<HumanReviewAnalyticsFilter>();
    assert_type::<HumanReviewRepo>();
    assert_type::<KnowledgeFileRow>();
    assert_type::<KnowledgeRepo>();
    assert_type::<KnowledgeSourceRow>();
    assert_type::<NewKnowledgeFile>();
    assert_type::<NewKnowledgeSource>();
    assert_type::<PolicyRepo>();
    assert_type::<PolicyRow>();
    assert_type::<PostgresStore>();
    assert_type::<ResolvedGatewayRoute>();
    assert_type::<RunFilter>();
    assert_type::<RunRepo>();
    assert_type::<SourceLabelPolicyRepo>();
    assert_type::<StoredFinancialAction>();
    assert_type::<StoredFinancialActionEvent>();
    assert_type::<StoredFinancialActionOutcome>();
    assert_type::<StoredFinancialApprovalRequest>();
    assert_type::<StoredFinancialMandate>();
    assert_type::<StoredFinancialReceipt>();
    assert_type::<StoredSourceLabelPolicy>();
    assert_type::<StoredToolMetadata>();
    assert_type::<TeamRepo>();
    assert_type::<ToolMetadataRepo>();
    assert_type::<TraceRepo>();
    assert_type::<TraceRow>();
    assert_type::<TraceWrite>();
    assert_type::<UserRecord>();
    assert_type::<UserRepo>();
    assert_type::<WriterConfig>();
    assert_type::<AddMemberOutcome>();

    let _connect = connect_postgres;
    let _migrate = migrate_postgres;
    let _spawn = spawn_writer;
    let _schema = schema::agents::table;
}
