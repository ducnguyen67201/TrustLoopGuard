ALTER TABLE workspaces
    ADD COLUMN is_knowledge_base_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN is_attacks_enabled BOOLEAN NOT NULL DEFAULT false;
