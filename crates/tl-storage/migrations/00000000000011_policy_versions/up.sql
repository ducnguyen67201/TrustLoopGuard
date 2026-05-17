-- Generic version history for any workspace entity (policy, agent, knowledge_source, …).
-- entity_type is the owning entity kind; entity_id is its primary key.
-- No FK constraint: polymorphic reference enforced at the application layer.
CREATE TABLE entity_versions (
    workspace_id TEXT        NOT NULL,
    entity_type  TEXT        NOT NULL,
    entity_id    TEXT        NOT NULL,
    version      INTEGER     NOT NULL,
    content      TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, entity_type, entity_id, version)
);

CREATE INDEX entity_versions_list_idx
    ON entity_versions (workspace_id, entity_type, entity_id, version DESC);
