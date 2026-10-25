//! Automated unit tests for transaction ledger settlement.
//!
//! // @trace SPEC-DEMO-CAPTURE-SVC, REQ-DEMO-CAPTURE

#[path = "../src/ledger.rs"]
mod ledger;

use ledger::{capture_transaction, LedgerEntry, LedgerError};

#[test]
// @trace SPEC-DEMO-CAPTURE-SVC, REQ-DEMO-CAPTURE
fn test_capture_balanced_transaction() {
    let entries = vec![
        LedgerEntry {
            account_id: "acct_buyer_123".to_string(),
            amount_cents: 5000,
            is_credit: false, // debit buyer
        },
        LedgerEntry {
            account_id: "acct_merchant_456".to_string(),
            amount_cents: 5000,
            is_credit: true, // credit merchant
        },
    ];

    assert!(capture_transaction(&entries).is_ok());
}

#[test]
// @trace SPEC-DEMO-CAPTURE-SVC, REQ-DEMO-CAPTURE
fn test_capture_unbalanced_transaction_fails() {
    let entries = vec![
        LedgerEntry {
            account_id: "acct_buyer_123".to_string(),
            amount_cents: 5000,
            is_credit: false,
        },
        LedgerEntry {
            account_id: "acct_merchant_456".to_string(),
            amount_cents: 4500,
            is_credit: true,
        },
    ];

    assert_eq!(
        capture_transaction(&entries),
        Err(LedgerError::UnbalancedTransaction {
            debits: 5000,
            credits: 4500,
        })
    );
}
