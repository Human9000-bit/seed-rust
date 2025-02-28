use crate::base64::{decode_base64, encode_base64};
use crate::seed::entity::message::{self, OutcomeMessage};
use crate::seed::error::SeedError;
use crate::traits::message::MessagesDB;
use anyhow::{Result, anyhow};
use base64::prelude::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, query};
use std::env::var;
use thiserror::Error;

/// Represents a PostgreSQL database connection pool
///
/// This struct wraps a SQLx connection pool for Postgres and provides
/// methods for database operations.
#[derive(Clone)]
pub struct PostgresDatabase {
    /// The underlying connection pool to the Postgres database
    pub db: Pool<Postgres>,
}

impl PostgresDatabase {
    /// Creates a new PostgresDatabase instance with a connection pool
    ///
    /// # Returns
    /// - `Result<Self>` - A new PostgresDatabase instance wrapped in Result
    ///
    /// # Errors
    /// Will return an error if unable to establish database connection
    ///
    /// # Environment Variables
    /// - `DB_USER` - Database username (default: "postgres")
    /// - `DB_PASSWORD` - Database password (default: "mysecretpassword")
    /// - `DB_NAME` - Database name (default: "postgres")
    pub async fn new() -> Result<Self> {
        // Try to get database username from environment, fall back to default if unset
        let db_user = var("DB_USER")
            .inspect_err(|_| warn!("DB_USER environment variable is unset, using default..."))
            .unwrap_or("postgres".to_string());

        // Try to get database password from environment, fall back to default if unset
        let db_password = var("DB_PASSWORD")
            .inspect_err(|_| warn!("DB_PASSWORD environment variable is unset, using default..."))
            .unwrap_or("mysecretpassword".to_string());

        // Try to get database name from environment, fall back to default if unset
        let db_name = var("DB_NAME")
            .inspect_err(|_| warn!("DB_NAME environment variable is unset, using default..."))
            .unwrap_or("seed-rust".to_string());

        // Construct the Postgres connection URL
        let connection_url = format!("postgres://{db_user}:{db_password}@localhost:5432/{db_name}");

        // Create and connect to the database pool
        let pool = PgPoolOptions::new()
            .connect(&connection_url)
            .await
            .inspect_err(|e| error!("failed to connect to postgres pool: {e}"))?;

        Ok(Self { db: pool })
    }

    /// Retrieves the highest nonce value for a given chat ID from the database
    ///
    /// # Arguments
    /// * `chat_id` - Binary chat identifier to search for
    ///
    /// # Returns
    /// * `Result<usize>` - Highest known nonce or 0 if none exist
    ///
    /// # Errors
    /// Returns errors for:
    /// - Database query failures
    /// - Missing chat history (NotFound)
    async fn get_last_nonce(&self, chat_id: &[u8]) -> Result<usize> {
        let chat_id = ByteSeq(chat_id);

        // Query for maximum nonce using parameterized SQL
        let last_nonce = sqlx::query!(
            r#"
                    SELECT MAX(nonce)
                    FROM messages
                    WHERE chat_id = $1"#,
            chat_id as ByteSeq
        );

        // Execute query and process results
        let last_nonce = last_nonce.fetch_one(&self.db).await?;
        match last_nonce.max {
            Some(int) => Ok(int as usize),
            None => Err(anyhow!(DatabaseError::NotFound)),
        }
    }
}

impl MessagesDB for PostgresDatabase {
    /// Inserts a new message into the database after validating and processing fields
    ///
    /// # Arguments
    /// * `message` - The incoming message containing encrypted content and metadata
    ///
    /// # Returns
    /// * `Result<()>` - Empty result indicating success or failure
    ///
    /// # Errors
    /// Returns errors for:
    /// - Base64 decoding failures
    /// - Nonce validation failures
    /// - Database insertion errors
    /// - Invalid sequence of nonces
    async fn insert_message(&self, message: message::Message) -> Result<()> {
        // Decode base64 encoded chat ID from message
        let chat_id = BASE64_STANDARD
            .decode(message.chat_id)
            .inspect_err(|e| error!("invalid message: {e}"))?;

        // Decode base64 encoded signature using helper function
        let signature = decode_base64(message.signature).await?;

        // Start async fetch of last known nonce for this chat
        let last_nonce_future = self.get_last_nonce(chat_id.as_slice());

        // Parallel decode of content and initialization vector
        let content = decode_base64(message.content).await?;
        let content_iv = decode_base64(message.content_iv).await?;

        // Await completion of nonce query
        let last_nonce = last_nonce_future.await?;

        // Validate sequential nonce increment
        if message.nonce != last_nonce + 1 {
            return Err(anyhow!(SeedError::InvalidNonce));
        }

        // Prepare SQL parameters with dedicated types for type safety
        let last_nonce = DBInt(last_nonce as i64);
        let chat_id = ByteSeq(&chat_id);
        let signature = ByteSeq(&signature);
        let content = ByteSeq(&content);
        let content_iv = ByteSeq(&content_iv);

        // Execute parameterized SQL insert query
        query!(
            r#"
                INSERT INTO messages (nonce, chat_id, signature, content, content_iv)
                VALUES ($1, $2, $3, $4, $5)
            "#,
            last_nonce as DBInt,
            chat_id as ByteSeq,
            signature as ByteSeq,
            content as ByteSeq,
            content_iv as ByteSeq
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Fetches message history for a given chat from the database
    ///
    /// # Arguments
    /// * `chat_id` - Binary chat identifier to fetch messages for
    /// * `nonce` - Starting nonce value for history fetch
    /// * `amount` - Maximum number of messages to retrieve
    ///
    /// # Returns
    /// * `Result<OutcomeMessage>` - Retrieved messages wrapped in Result
    ///
    /// # Errors
    /// - Database query failures
    /// - Data conversion errors
    async fn fetch_history(
        &self,
        chat_id: &[u8],
        nonce: usize,
        amount: usize,
    ) -> Result<Vec<OutcomeMessage>> {
        // Convert parameters to DB-compatible types
        let chat_id = ByteSeq(chat_id);
        let nonce = DBInt(nonce as i64);
        let amount = DBInt(amount as i64);

        // Execute SQL query to fetch message history
        // Uses type annotations to ensure correct column types
        // Filters by chat_id and nonce, orders ascending, limits results
        let rows = sqlx::query!(
            r#"
                SELECT
                    nonce as "nonce!: i64",
                    chat_id as "chat_id!: Vec<u8>",
                    signature as "signature!: Vec<u8>",
                    content as "content!: Vec<u8>",
                    content_iv as "content_iv!: Vec<u8>"
                FROM messages
                WHERE chat_id = $1 AND nonce >= $2
                ORDER BY nonce ASC
                LIMIT $3
            "#,
            chat_id as ByteSeq,
            nonce as DBInt,
            amount as DBInt
        );

        // Fetch all matching rows from database
        let rows = rows.fetch_all(&self.db).await?;

        // Pre-allocate vector to hold converted messages
        let mut messages: Vec<OutcomeMessage> = Vec::with_capacity(rows.len());

        // Convert each database row into an OutcomeMessage
        for row in rows {
            // Convert numeric nonce to usize
            let nonce = row.nonce as usize;

            // Base64 encode all binary fields
            let chat_id: String = encode_base64(row.chat_id.as_slice()).await;
            let signature: String = encode_base64(row.signature.as_slice()).await;
            let content: String = encode_base64(row.content.as_slice()).await;
            let content_iv: String = encode_base64(row.chat_id.as_slice()).await;

            // Construct OutcomeMessage from encoded fields
            let message = OutcomeMessage {
                nonce,
                chat_id,
                signature,
                content,
                content_iv,
            };

            messages.push(message);
        }

        Ok(messages)
    }
}

/// SQLx compatible wrapper for byte sequence parameters
///
/// Allows proper type handling when passing binary data to PostgreSQL
#[derive(sqlx::Type, Debug)]
#[sqlx(transparent)]
struct ByteSeq<'a>(&'a [u8]);

/// SQLx compatible wrapper for integer parameters
///
/// Ensures proper type mapping between Rust and PostgreSQL
#[derive(sqlx::Type, Debug)]
#[sqlx(transparent)]
struct DBInt(i64);

/// Database operation error types
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Indicates missing database records when expected
    #[error("query not found in the database")]
    NotFound,

    /// Indicates failure to insert new database record
    #[error("failed to insert the message")]
    InsertError,

    /// Indicates failure to convert sql rows into [IncomeMessage]s
    #[error("failed to prepare data for history fetch")]
    FetchHistoryDataPrepareError,
}
