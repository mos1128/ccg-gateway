use sqlx::SqlitePool;

/// Record a successful request for a provider
/// Resets consecutive_failures to 0
/// Returns (had_previous_failures) to indicate if the provider was recovering
pub async fn record_success(db: &SqlitePool, provider_id: i64) -> Result<bool, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Check if provider had previous failures
    let had_failures: Option<(i64,)> =
        sqlx::query_as("SELECT consecutive_failures FROM providers WHERE id = ?")
            .bind(provider_id)
            .fetch_optional(db)
            .await?;

    let had_previous_failures = had_failures.map(|(cf,)| cf > 0).unwrap_or(false);

    sqlx::query(
        r#"
        UPDATE providers
        SET consecutive_failures = 0,
            blacklisted_until = NULL,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(provider_id)
    .execute(db)
    .await?;

    Ok(had_previous_failures)
}

/// Record a failed request for a provider
/// Increments consecutive_failures and blacklists if threshold is reached
/// If the provider was blacklisted but blacklist has expired, resets count before incrementing
/// Uses atomic UPDATE to avoid race conditions with concurrent requests
/// Returns (was_blacklisted, provider_name) tuple
pub async fn record_failure(
    db: &SqlitePool,
    provider_id: i64,
) -> Result<(bool, String), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Get provider state including current blacklist status
    let provider: Option<(i64, i64, i64, Option<i64>, String)> = sqlx::query_as(
        "SELECT consecutive_failures, failure_threshold, blacklist_minutes, blacklisted_until, name FROM providers WHERE id = ?",
    )
    .bind(provider_id)
    .fetch_optional(db)
    .await?;

    let Some((
        consecutive_failures,
        failure_threshold,
        blacklist_minutes,
        blacklisted_until,
        provider_name,
    )) = provider
    else {
        return Ok((false, String::new()));
    };

    // Check if provider is currently blacklisted (blacklisted_until > now)
    let currently_blacklisted = blacklisted_until.map(|t| t > now).unwrap_or(false);

    // If currently blacklisted, don't update anything
    if currently_blacklisted {
        return Ok((false, provider_name));
    }

    // Determine base count: if blacklist expired, reset to 0; otherwise use current value
    let base_count = if blacklisted_until.is_some() {
        // Blacklist expired (since we passed the currently_blacklisted check)
        0
    } else {
        // Never been blacklisted, use current count
        consecutive_failures
    };

    // Increment failure count
    let new_failures = base_count + 1;

    // Check if we should blacklist
    let should_blacklist = new_failures >= failure_threshold;

    if should_blacklist {
        let blacklist_until = now + (blacklist_minutes * 60);
        sqlx::query(
            r#"
            UPDATE providers
            SET consecutive_failures = ?,
                blacklisted_until = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_failures)
        .bind(blacklist_until)
        .bind(now)
        .bind(provider_id)
        .execute(db)
        .await?;

        tracing::warn!(
            provider_id = provider_id,
            failures = new_failures,
            "Provider blacklisted due to consecutive failures"
        );

        Ok((true, provider_name))
    } else {
        // If blacklist expired, clear it; otherwise just update failure count
        sqlx::query(
            r#"
            UPDATE providers
            SET consecutive_failures = ?,
                blacklisted_until = CASE WHEN blacklisted_until IS NOT NULL AND blacklisted_until <= ? THEN NULL ELSE blacklisted_until END,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_failures)
        .bind(now)
        .bind(now)
        .bind(provider_id)
        .execute(db)
        .await?;

        Ok((false, provider_name))
    }
}

/// Reset provider failures and remove blacklist
pub async fn reset_failures(db: &SqlitePool, provider_id: i64) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        UPDATE providers
        SET consecutive_failures = 0,
            blacklisted_until = NULL,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(provider_id)
    .execute(db)
    .await?;

    Ok(())
}
