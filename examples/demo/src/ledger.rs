//! Double-entry transaction ledger settlement engine.
//!
//! // @trace SPEC-DEMO-CAPTURE-SVC, REQ-DEMO-CAPTURE

/// Represents a single double-entry ledger entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    pub account_id: String,
    pub amount_cents: i64,
    pub is_credit: bool,
}

/// Ledger settlement error types.
#[derive(Debug, PartialEq, Eq)]
pub enum LedgerError {
    UnbalancedTransaction { debits: i64, credits: i64 },
    ZeroAmount,
}

/// Validates and captures an atomic double-entry transaction.
///
/// // @trace SPEC-DEMO-CAPTURE-SVC, REQ-DEMO-CAPTURE
pub fn capture_transaction(entries: &[LedgerEntry]) -> Result<(), LedgerError> {
    if entries.is_empty() {
        return Err(LedgerError::ZeroAmount);
    }

    let mut total_debits = 0i64;
    let mut total_credits = 0i64;

    for entry in entries {
        if entry.amount_cents <= 0 {
            return Err(LedgerError::ZeroAmount);
        }
        if entry.is_credit {
            total_credits = total_credits.saturating_add(entry.amount_cents);
        } else {
            total_debits = total_debits.saturating_add(entry.amount_cents);
        }
    }

    if total_debits != total_credits {
        return Err(LedgerError::UnbalancedTransaction {
            debits: total_debits,
            credits: total_credits,
        });
    }

    Ok(())
}
