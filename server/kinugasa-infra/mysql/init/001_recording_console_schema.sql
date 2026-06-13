CREATE TABLE IF NOT EXISTS mocap_teams (
    id CHAR(36) NOT NULL,
    external_usergroup_key VARCHAR(255) NOT NULL,
    created_at DATETIME(6) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY mocap_teams_external_usergroup_key_unique (external_usergroup_key),
    KEY mocap_teams_created_at_id_index (created_at, id)
);

CREATE TABLE IF NOT EXISTS studios (
    id CHAR(36) NOT NULL,
    team_id CHAR(36) NOT NULL,
    name TEXT NOT NULL,
    status VARCHAR(32) NOT NULL,
    last_event_sequence_number BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    PRIMARY KEY (id),
    KEY studios_team_created_at_id_index (team_id, created_at, id),
    CONSTRAINT studios_team_id_fk
        FOREIGN KEY (team_id)
        REFERENCES mocap_teams (id),
    CONSTRAINT studios_status_check
        CHECK (status IN ('capturing', 'idle', 'closed'))
);
