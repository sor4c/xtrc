//! Integration tests for specification document parser.
//!
//! // @trace SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE, SPEC-FMT-GRAMMAR

use std::path::Path;
use xtrc::error::ParseError;
use xtrc::model::Layer;
use xtrc::parser::document::{parse_document, parse_document_file, parse_spec_file};

#[test]
fn test_parse_actual_repository_requirements_md() {
    let path = Path::new("docs/requirements.md");
    let doc = parse_document_file(path).expect("parse docs/requirements.md");

    assert_eq!(doc.frontmatter.document_type, "requirements_specification");
    assert_eq!(doc.frontmatter.id, "REQ-SPEC-001");
    assert_eq!(doc.frontmatter.status, "approved");
    assert_eq!(doc.frontmatter.version, "1.0.0");

    // Must parse all 49 specification nodes
    assert_eq!(
        doc.nodes.len(),
        49,
        "Expected exactly 49 nodes in docs/requirements.md"
    );

    // Verify first node: VIS-VCS-NATIVE at line 27
    let first = &doc.nodes[0];
    assert_eq!(first.id().as_str(), "VIS-VCS-NATIVE");
    assert_eq!(
        first.title(),
        "Fast, Plain-Text Traceability Native to Version Control"
    );
    assert_eq!(first.layer(), Layer::Vision);
    assert_eq!(first.file_origin().line(), 27);
    assert!(first.parents().is_empty());

    // Verify layer counts
    let vision_count = doc
        .nodes
        .iter()
        .filter(|n| n.layer() == Layer::Vision)
        .count();
    let strat_count = doc
        .nodes
        .iter()
        .filter(|n| n.layer() == Layer::Strategy)
        .count();
    let epic_count = doc
        .nodes
        .iter()
        .filter(|n| n.layer() == Layer::Epic)
        .count();
    let us_count = doc
        .nodes
        .iter()
        .filter(|n| n.layer() == Layer::UserStory)
        .count();
    let req_count = doc
        .nodes
        .iter()
        .filter(|n| n.layer() == Layer::Requirement)
        .count();

    assert_eq!(vision_count, 1);
    assert_eq!(strat_count, 4);
    assert_eq!(epic_count, 6);
    assert_eq!(us_count, 17);
    assert_eq!(req_count, 21);

    // Test convenience function parse_spec_file produces same nodes
    let nodes = parse_spec_file(path).expect("parse_spec_file");
    assert_eq!(nodes.len(), 49);
}

#[test]
fn test_parse_actual_repository_design_md() {
    let path = Path::new("docs/design.md");
    let doc = parse_document_file(path).expect("parse docs/design.md");

    assert_eq!(doc.frontmatter.document_type, "design_specification");
    assert_eq!(doc.frontmatter.id, "DES-SPEC-001");
    assert_eq!(doc.frontmatter.status, "approved");

    // Must parse all 16 technical specification nodes
    assert_eq!(
        doc.nodes.len(),
        16,
        "Expected exactly 16 SPEC-* nodes in docs/design.md"
    );

    // Verify first node: SPEC-DISC-INIT at line 57
    let first = &doc.nodes[0];
    assert_eq!(first.id().as_str(), "SPEC-DISC-INIT");
    assert_eq!(
        first.title(),
        "Workspace Initialization & Configuration Scaffolding"
    );
    assert_eq!(first.layer(), Layer::TechnicalSpec);
    assert_eq!(first.file_origin().line(), 57);

    // Verify all 16 nodes are TechnicalSpec
    for node in &doc.nodes {
        assert_eq!(
            node.layer(),
            Layer::TechnicalSpec,
            "Node '{}' should have layer TechnicalSpec",
            node.id()
        );
    }
}

#[test]
fn test_frontmatter_error_exit_codes_and_validation() {
    let missing_fences = "# No frontmatter at all\n### REQ-01\n";
    match parse_document(missing_fences, Path::new("bad.md")) {
        Err(err @ ParseError::InvalidFrontmatter { .. }) => {
            assert_eq!(err.exit_code(), 2);
        }
        other => panic!("Expected InvalidFrontmatter, got {:?}", other),
    }

    let unclosed_fence = "---\ndocument_type: test\n# No closing fence\n";
    match parse_document(unclosed_fence, Path::new("bad.md")) {
        Err(err @ ParseError::InvalidFrontmatter { .. }) => {
            assert_eq!(err.exit_code(), 2);
        }
        other => panic!("Expected InvalidFrontmatter, got {:?}", other),
    }

    let missing_field = "---\ndocument_type: test\nid: TST-01\ntitle: Test\nversion: 1.0\n---\n";
    match parse_document(missing_field, Path::new("bad.md")) {
        Err(err @ ParseError::InvalidFrontmatter { .. }) => {
            assert_eq!(err.exit_code(), 2);
            assert!(err.to_string().contains("status"));
        }
        other => panic!("Expected InvalidFrontmatter, got {:?}", other),
    }
}

#[test]
fn test_heading_and_parent_variations() {
    let content = r#"---
document_type: test_spec
id: TST-SPEC-001
title: Test Specification
version: 1.0.0
status: approved
---

### REQ-STANDALONE
- **ID**: REQ-STANDALONE
- **Parent**: None
- **Type**: Requirement

#### SPEC-SUB-01: Sub-level 4 node
- **ID**: SPEC-SUB-01
- **Parent**: REQ-STANDALONE
- **Type**: TechnicalSpec
"#;

    let doc = parse_document(content, Path::new("test.md")).expect("parse");
    assert_eq!(doc.nodes.len(), 2);

    assert_eq!(doc.nodes[0].id().as_str(), "REQ-STANDALONE");
    assert_eq!(doc.nodes[0].title(), "");
    assert_eq!(doc.nodes[0].parents().len(), 0);

    assert_eq!(doc.nodes[1].id().as_str(), "SPEC-SUB-01");
    assert_eq!(doc.nodes[1].title(), "Sub-level 4 node");
    assert_eq!(doc.nodes[1].parents().len(), 1);
    assert_eq!(doc.nodes[1].parents()[0].as_str(), "REQ-STANDALONE");
}

#[test]
fn test_adversarial_frontmatter_edge_cases() {
    let dummy_path = Path::new("test.md");

    // 1. Empty and whitespace
    let err = parse_document("", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    let err = parse_document("   \n\t  \n   ", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 2. Unclosed opening fence
    let err = parse_document("---\ndocument_type: req\n# Content", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));
    assert!(err.to_string().contains("Unclosed YAML frontmatter"));

    // 3. Single fence without content
    let err = parse_document("---", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 4. Missing opening fence (starts directly with fields)
    let err = parse_document("document_type: req\nid: REQ-01\n---\n", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 5. Body only without frontmatter
    let err = parse_document("# Title\nSome content without frontmatter", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 6. Blank line before frontmatter
    let err = parse_document("\n---\ndocument_type: req\n---\n", dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 7. Invalid YAML syntax
    let err = parse_document(
        "---\ndocument_type: req\nid: [unclosed bracket\n---\n",
        dummy_path,
    )
    .unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));

    // 8. Missing mandatory fields individually
    let base = "document_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: draft\n";
    for mandatory_key in ["document_type", "id", "title", "version", "status"] {
        let lines: Vec<&str> = base
            .lines()
            .filter(|l| !l.starts_with(mandatory_key))
            .collect();
        let content = format!("---\n{}\n---\n", lines.join("\n"));
        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(
            matches!(err, ParseError::InvalidFrontmatter { .. }),
            "Expected InvalidFrontmatter for missing {}",
            mandatory_key
        );
    }

    // 9. Empty mandatory fields individually
    for mandatory_key in ["document_type", "id", "title", "version", "status"] {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: draft\n{}: \"   \"\n---\n",
            mandatory_key
        );
        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(
            matches!(err, ParseError::InvalidFrontmatter { .. }),
            "Expected InvalidFrontmatter for empty {}",
            mandatory_key
        );
    }

    // 10. Invalid status values
    for invalid_status in [
        "published",
        "review",
        "active",
        "rejected",
        "unknown",
        "123",
    ] {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: {}\n---\n",
            invalid_status
        );
        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(matches!(err, ParseError::InvalidFrontmatter { .. }));
        assert!(err.to_string().contains("Invalid document status"));
    }

    // 11. Valid statuses with casing variants
    for valid_status in [
        "draft",
        "Draft",
        "DRAFT",
        "approved",
        "Approved",
        "APPROVED",
        "deprecated",
        "Deprecated",
        "DEPRECATED",
        "  draft  ",
    ] {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: {}\n---\n",
            valid_status
        );
        let doc = parse_document(&content, dummy_path).expect("valid status");
        assert!(
            doc.frontmatter.status == "draft"
                || doc.frontmatter.status == "approved"
                || doc.frontmatter.status == "deprecated"
        );
        assert_eq!(doc.nodes.len(), 0);
    }

    // 12. Document with frontmatter only (no body)
    let content = "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\n---\n";
    let doc = parse_document(content, dummy_path).expect("parse frontmatter only");
    assert_eq!(doc.nodes.len(), 0);
    assert_eq!(doc.frontmatter.id, "REQ-01");

    // 13. Extra frontmatter fields preserved in extra map
    let content = "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\nschema_version: \"2.0\"\ngovernance:\n  issue: 5\n---\n";
    let doc = parse_document(content, dummy_path).expect("parse extra fields");
    assert!(doc.frontmatter.extra.contains_key("schema_version"));
    assert!(doc.frontmatter.extra.contains_key("governance"));

    // 14. Multiple fences in document (frontmatter + body horizontal rules)
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

---

### REQ-HR: Node after horizontal rule
- **ID**: REQ-HR
- **Parent**: None

---

Body divider
"#;
    let doc = parse_document(content, dummy_path).expect("parse with horizontal rules");
    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].id().as_str(), "REQ-HR");
}

#[test]
fn test_adversarial_headings_and_code_blocks() {
    let dummy_path = Path::new("test.md");

    // 1. Code blocks with ```, ~~~, and ````
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

### REQ-REAL-01: Real Node One
- **ID**: REQ-REAL-01
- **Parent**: None

```markdown
### REQ-FAKE-01: Fake Heading in Backticks
- **ID**: REQ-FAKE-01
```

~~~markdown
### REQ-FAKE-02: Fake Heading in Tildes
- **ID**: REQ-FAKE-02
~~~

````markdown
### REQ-FAKE-03: Fake Heading in Quadruple Backticks
- **ID**: REQ-FAKE-03
````

```
### req_malformed: Malformed ID in Code Block
```

### REQ-REAL-02: Real Node Two
- **ID**: REQ-REAL-02
- **Parent**: REQ-REAL-01
"#;

    let doc = parse_document(content, dummy_path).expect("parse code blocks");
    assert_eq!(doc.nodes.len(), 2, "Only real nodes should be extracted");
    assert_eq!(doc.nodes[0].id().as_str(), "REQ-REAL-01");
    assert_eq!(doc.nodes[1].id().as_str(), "REQ-REAL-02");
    // Verify descriptions contain code block contents
    assert!(doc.nodes[0].description().contains("REQ-FAKE-01"));
    assert!(doc.nodes[0].description().contains("REQ-FAKE-02"));
    assert!(doc.nodes[0].description().contains("REQ-FAKE-03"));
    assert!(doc.nodes[0].description().contains("req_malformed"));

    // 2. Extra whitespace around headings
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

###    REQ-WS-01:    Lots of spaces in title    
- **ID**: REQ-WS-01
- **Parent**: None

####   SPEC-WS-02:   Level 4 spaced   
- **ID**: SPEC-WS-02
- **Parent**: REQ-WS-01
"#;
    let doc = parse_document(content, dummy_path).expect("parse spaced headings");
    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.nodes[0].id().as_str(), "REQ-WS-01");
    assert_eq!(doc.nodes[0].title(), "Lots of spaces in title");
    assert_eq!(doc.nodes[1].id().as_str(), "SPEC-WS-02");
    assert_eq!(doc.nodes[1].title(), "Level 4 spaced");

    // 3. Headings without titles
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

### REQ-NO-TITLE
- **ID**: REQ-NO-TITLE
- **Parent**: None

### REQ-EMPTY-COLON:
- **ID**: REQ-EMPTY-COLON
- **Parent**: None

### REQ-SPACES-COLON:     
- **ID**: REQ-SPACES-COLON
- **Parent**: None
"#;
    let doc = parse_document(content, dummy_path).expect("parse headings without titles");
    assert_eq!(doc.nodes.len(), 3);
    assert_eq!(doc.nodes[0].title(), "");
    assert_eq!(doc.nodes[1].title(), "");
    assert_eq!(doc.nodes[2].title(), "");

    // 4. Invalid heading IDs (attempted IDs must fail with GrammarViolation and exit code 2)
    let invalid_headings = [
        "### req-lowercase: Lowercase Prefix",
        "### REQ_underscore: Underscore ID",
        "### REQ-trailing-: Trailing Hyphen",
        "### REQ--double: Double Hyphen",
        "### REQ-bad;token: Bad Token",
        "### SPEC_bad: Spec Underscore",
        "### US_bad: US Underscore",
        "### EPIC_bad: Epic Underscore",
        "### STRAT_bad: Strat Underscore",
        "### VIS_bad: Vis Underscore",
        "### REQ-01.1: Dot in segment",
    ];

    for invalid_h in invalid_headings {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\n---\n\n{}\n- **Parent**: None\n",
            invalid_h
        );
        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(
            err.exit_code(),
            2,
            "Expected exit code 2 for invalid heading: {}",
            invalid_h
        );
        assert!(
            matches!(err, ParseError::GrammarViolation { .. }),
            "Expected GrammarViolation for invalid heading: {}",
            invalid_h
        );
    }

    // 5. Non-attempted regular markdown headings should be ignored / preserved in body
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

### REQ-REAL-03: Real Node
- **ID**: REQ-REAL-03
- **Parent**: None

### 1. Introduction and Section Overview
### Requirements Architecture
### Note on System Performance
Some descriptive text.
"#;
    let doc = parse_document(content, dummy_path).expect("parse regular headings");
    assert_eq!(doc.nodes.len(), 1);
    assert!(doc.nodes[0]
        .description()
        .contains("### 1. Introduction and Section Overview"));
    assert!(doc.nodes[0]
        .description()
        .contains("### Requirements Architecture"));

    // 6. Section H1 and H2 finalize active nodes
    let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

### REQ-REAL-04: Before H1
- **ID**: REQ-REAL-04
- **Parent**: None

Text in node 4.

# New Major Section

Text outside any node.

## Subsection

### REQ-REAL-05: After H1 and H2
- **ID**: REQ-REAL-05
- **Parent**: None

Text in node 5.
"#;
    let doc = parse_document(content, dummy_path).expect("parse with H1/H2");
    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.nodes[0].id().as_str(), "REQ-REAL-04");
    assert!(!doc.nodes[0].description().contains("New Major Section"));
    assert_eq!(doc.nodes[1].id().as_str(), "REQ-REAL-05");
}

#[test]
fn test_adversarial_parent_attribute_variations() {
    let dummy_path = Path::new("test.md");

    // 1. Empty parent formats
    let empty_parent_bullets = [
        "- **Parent**: ",
        "- **Parent**: None",
        "- **Parent**: none",
        "- **Parent**: NONE",
        "- **Parents**: []",
        "- **Parents**: [none]",
        "- **Parents**: [ None , NONE ]",
        "- **Parent Requirements**: None",
    ];

    for parent_bullet in empty_parent_bullets {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\n---\n\n### REQ-TEST: Test Node\n- **ID**: REQ-TEST\n{}\n",
            parent_bullet
        );
        let doc = parse_document(&content, dummy_path).expect("empty parent parse");
        assert_eq!(
            doc.nodes[0].parents().len(),
            0,
            "Expected 0 parents for: {}",
            parent_bullet
        );
    }

    // 2. Bracketed and unbracketed, quotes, spaces, trailing commas
    let content = r#"---
document_type: design_spec
id: DES-01
title: Title
version: 1.0.0
status: approved
---

### SPEC-SYNTAX-01: Single Unbracketed
- **ID**: SPEC-SYNTAX-01
- **Parent**: REQ-01

### SPEC-SYNTAX-02: Multiple Unbracketed
- **ID**: SPEC-SYNTAX-02
- **Parents**: REQ-01, REQ-02, REQ-03

### SPEC-SYNTAX-03: Single Bracketed
- **ID**: SPEC-SYNTAX-03
- **Parents**: [REQ-01]

### SPEC-SYNTAX-04: Bracketed with Quoted Strings
- **ID**: SPEC-SYNTAX-04
- **Parents**: ["REQ-01", 'REQ-02', "REQ-03"]

### SPEC-SYNTAX-05: Trailing and Consecutive Commas
- **ID**: SPEC-SYNTAX-05
- **Parents**: [REQ-01, , REQ-02,]
"#;
    let doc = parse_document(content, dummy_path).expect("parent syntax variations");
    assert_eq!(doc.nodes.len(), 5);
    assert_eq!(doc.nodes[0].parents().len(), 1);
    assert_eq!(doc.nodes[1].parents().len(), 3);
    assert_eq!(doc.nodes[2].parents().len(), 1);
    assert_eq!(doc.nodes[3].parents().len(), 3);
    assert_eq!(doc.nodes[4].parents().len(), 2);

    // 3. Multiple parent lines under one heading with deduplication
    let content = r#"---
document_type: design_spec
id: DES-01
title: Title
version: 1.0.0
status: approved
---

### SPEC-MULTI-PARENT: Multiple Parent Lines
- **ID**: SPEC-MULTI-PARENT
- **Parent**: REQ-01
- **Parents**: [REQ-02, REQ-03]
- **Parent Requirements**: REQ-04, REQ-01
"#;
    let doc = parse_document(content, dummy_path).expect("multi parent lines");
    assert_eq!(doc.nodes.len(), 1);
    let parents = doc.nodes[0].parents();
    assert_eq!(parents.len(), 4, "Should deduplicate REQ-01 to exactly 4");
    assert_eq!(parents[0].as_str(), "REQ-01");
    assert_eq!(parents[1].as_str(), "REQ-02");
    assert_eq!(parents[2].as_str(), "REQ-03");
    assert_eq!(parents[3].as_str(), "REQ-04");

    // 4. Invalid parent IDs must fail with GrammarViolation and exit code 2
    let invalid_parents = [
        "- **Parent**: bad_parent",
        "- **Parents**: [REQ-01, bad_parent]",
        "- **Parents**: [REQ-01], REQ-02",
        "- **Parents**: REQ-01, 12345",
        "- **Parents**: REQ-01, REQ_02",
    ];

    for parent_bullet in invalid_parents {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\n---\n\n### REQ-FAIL: Node\n- **ID**: REQ-FAIL\n{}\n",
            parent_bullet
        );
        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(
            err.exit_code(),
            2,
            "Expected exit code 2 for invalid parent: {}",
            parent_bullet
        );
        assert!(
            matches!(err, ParseError::GrammarViolation { .. }),
            "Expected GrammarViolation for: {}",
            parent_bullet
        );
    }
}

#[test]
fn test_adversarial_provenance_line_numbers() {
    let dummy_path = Path::new("docs/test_provenance.md");

    let content = "---
document_type: requirements_specification
id: REQ-SPEC-001
title: Provenance Test Specification
version: 1.0.0
status: approved
---

Paragraph before first node.

### REQ-PROV-01: First Node
- **ID**: REQ-PROV-01
- **Parent**: None

Description line 1
Description line 2
```rust
let x = 1;
let y = 2;
```
Description line 3

### REQ-PROV-02: Second Node
- **ID**: REQ-PROV-02
- **Parent**: REQ-PROV-01

Description for second node.

### REQ-PROV-03: Third Node
- **ID**: REQ-PROV-03
- **Parent**: REQ-PROV-01
";

    let doc = parse_document(content, dummy_path).expect("parse provenance");
    assert_eq!(doc.nodes.len(), 3);

    // Verify exact lines of headings
    // Line 1: ---
    // Line 2: document_type: ...
    // Line 3: id: ...
    // Line 4: title: ...
    // Line 5: version: ...
    // Line 6: status: ...
    // Line 7: ---
    // Line 8:
    // Line 9: Paragraph before first node.
    // Line 10:
    // Line 11: ### REQ-PROV-01: First Node
    assert_eq!(doc.nodes[0].file_origin().line(), 11);
    assert_eq!(doc.nodes[0].file_origin().path(), dummy_path);

    // Line 23: ### REQ-PROV-02: Second Node
    assert_eq!(doc.nodes[1].file_origin().line(), 23);
    assert_eq!(doc.nodes[1].file_origin().path(), dummy_path);

    // Line 29: ### REQ-PROV-03: Third Node
    assert_eq!(doc.nodes[2].file_origin().line(), 29);
    assert_eq!(doc.nodes[2].file_origin().path(), dummy_path);

    // Verify line numbers in errors:
    // Error on invalid heading at line 12
    let error_content = "---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

Line 8
Line 9
Line 10
Line 11
### req_invalid: Heading with invalid ID
- **Parent**: None
";
    let err = parse_document(error_content, dummy_path).unwrap_err();
    match err {
        ParseError::GrammarViolation { line, token, .. } => {
            assert_eq!(line, 13, "Expected error on line 13");
            assert_eq!(token, "req_invalid");
        }
        other => panic!("Expected GrammarViolation, got {:?}", other),
    }

    // Error on invalid parent at line 14
    let error_parent = "---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: approved
---

Line 8
Line 9
Line 10
### REQ-PROV-ERR: Valid Heading
- **ID**: REQ-PROV-ERR
- **Parent**: invalid_parent_id
";
    let err = parse_document(error_parent, dummy_path).unwrap_err();
    match err {
        ParseError::GrammarViolation { line, token, .. } => {
            assert_eq!(line, 14, "Expected error on line 14");
            assert_eq!(token, "invalid_parent_id");
        }
        other => panic!("Expected GrammarViolation, got {:?}", other),
    }

    // Test Windows CRLF line endings preservation
    let crlf_content = content.replace('\n', "\r\n");
    let crlf_doc = parse_document(&crlf_content, dummy_path).expect("parse CRLF");
    assert_eq!(crlf_doc.nodes.len(), 3);
    assert_eq!(crlf_doc.nodes[0].file_origin().line(), 11);
    assert_eq!(crlf_doc.nodes[1].file_origin().line(), 23);
    assert_eq!(crlf_doc.nodes[2].file_origin().line(), 29);
}

#[test]
fn test_adversarial_layer_inference_and_type_overrides() {
    let dummy_path = Path::new("test.md");

    // Standard prefix inference
    let prefixes = [
        ("VIS-01", Layer::Vision),
        ("STRAT-01", Layer::Strategy),
        ("EPIC-01", Layer::Epic),
        ("US-01", Layer::UserStory),
        ("REQ-01", Layer::Requirement),
        ("SPEC-01", Layer::TechnicalSpec),
    ];

    for (id, expected_layer) in prefixes {
        let content = format!(
            "---\ndocument_type: req\nid: REQ-01\ntitle: Title\nversion: 1.0.0\nstatus: approved\n---\n\n### {}: Title\n- **ID**: {}\n- **Parent**: None\n",
            id, id
        );
        let doc = parse_document(&content, dummy_path).expect("standard prefix inference");
        assert_eq!(doc.nodes[0].layer(), expected_layer);
    }

    // Explicit type override with non-standard prefix
    let content = r#"---
document_type: custom_spec
id: CUST-01
title: Title
version: 1.0.0
status: approved
---

### ARCH-01: Architectural Component
- **ID**: ARCH-01
- **Parent**: None
- **Type**: Technical Spec

### SEC-01: Security Requirement
- **ID**: SEC-01
- **Parent**: None
- **Type**: Functional Requirement

### GOAL-01: Product Vision Goal
- **ID**: GOAL-01
- **Parent**: None
- **Type**: Vision
"#;
    let doc = parse_document(content, dummy_path).expect("parse explicit type overrides");
    assert_eq!(doc.nodes.len(), 3);
    assert_eq!(doc.nodes[0].layer(), Layer::TechnicalSpec);
    assert_eq!(doc.nodes[1].layer(), Layer::Requirement);
    assert_eq!(doc.nodes[2].layer(), Layer::Vision);

    // Non-standard prefix WITHOUT explicit type fails
    let content = r#"---
document_type: custom_spec
id: CUST-01
title: Title
version: 1.0.0
status: approved
---

### ARCH-02: Missing Type
- **ID**: ARCH-02
- **Parent**: None
"#;
    let err = parse_document(content, dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::GrammarViolation { .. }));
    assert!(err.to_string().contains("Cannot infer layer"));

    // Invalid explicit type fails
    let content = r#"---
document_type: custom_spec
id: CUST-01
title: Title
version: 1.0.0
status: approved
---

### ARCH-03: Invalid Type
- **ID**: ARCH-03
- **Parent**: None
- **Type**: NonExistentLayer
"#;
    let err = parse_document(content, dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, ParseError::GrammarViolation { .. }));
    assert!(err.to_string().contains("Unrecognized layer type"));
}

// ============================================================================
// Milestone 2 Iteration 2: Document Parser & Type Override Regression Suite
// ============================================================================

/// Verifies that explicit `- **Type**: TechnicalSpec` (PascalCase without space)
/// on non-standard prefixes (e.g. `ARCH-01`, `DB-01`, `API-01`) successfully
/// assigns `Layer::TechnicalSpec` without throwing a GrammarViolation.
///
/// // @trace SPEC-FMT-FRONTMATTER, SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_regression_parser_explicit_technicalspec_non_standard_prefix() {
    let dummy_path = Path::new("docs/architecture.md");
    let content = r#"---
document_type: architecture_specification
id: ARCH-SPEC-001
title: System Architecture Specification
version: 1.0.0
status: approved
---

### ARCH-01: Architectural Subsystem
- **ID**: ARCH-01
- **Parent**: None
- **Type**: TechnicalSpec

### DB-01: Storage Engine Subsystem
- **ID**: DB-01
- **Parent**: ARCH-01
- **Type**: TechnicalSpec

### API-01: Public Interface Gate
- **ID**: API-01
- **Parent**: ARCH-01
- **Type**: TechnicalSpec
"#;

    let doc = parse_document(content, dummy_path).expect(
        "Explicit - **Type**: TechnicalSpec on non-SPEC prefix must parse without grammar error",
    );

    assert_eq!(doc.nodes.len(), 3);
    assert_eq!(doc.nodes[0].id().as_str(), "ARCH-01");
    assert_eq!(doc.nodes[0].layer(), Layer::TechnicalSpec);
    assert_eq!(doc.nodes[0].parents().len(), 0);

    assert_eq!(doc.nodes[1].id().as_str(), "DB-01");
    assert_eq!(doc.nodes[1].layer(), Layer::TechnicalSpec);
    assert_eq!(doc.nodes[1].parents().len(), 1);
    assert_eq!(doc.nodes[1].parents()[0].as_str(), "ARCH-01");

    assert_eq!(doc.nodes[2].id().as_str(), "API-01");
    assert_eq!(doc.nodes[2].layer(), Layer::TechnicalSpec);
    assert_eq!(doc.nodes[2].parents().len(), 1);
    assert_eq!(doc.nodes[2].parents()[0].as_str(), "ARCH-01");
}

/// Verifies explicit `- **Type**:` resolution across all 6 specification layers
/// using non-standard prefixes that cannot rely on prefix auto-inference.
///
/// // @trace SPEC-FMT-FRONTMATTER, SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_regression_parser_explicit_type_all_six_specification_layers() {
    let dummy_path = Path::new("docs/lifecycle_test.md");
    let content = r#"---
document_type: full_lifecycle_spec
id: LIFE-SPEC-001
title: Full Lifecycle Specification
version: 1.0.0
status: approved
---

### GOAL-01: North Star Product Vision
- **ID**: GOAL-01
- **Parent**: None
- **Type**: Vision

### ROAD-01: Strategic Market Expansion
- **ID**: ROAD-01
- **Parent**: GOAL-01
- **Type**: Strategy

### CAP-01: Core Capability Delivery
- **ID**: CAP-01
- **Parent**: ROAD-01
- **Type**: Epic

### PERSONA-01: Developer Fast Verification
- **ID**: PERSONA-01
- **Parent**: CAP-01
- **Type**: UserStory

### RULE-01: Deterministic Exit Codes
- **ID**: RULE-01
- **Parent**: PERSONA-01
- **Type**: Requirement

### IMPL-01: Exit Code Mapping Engine
- **ID**: IMPL-01
- **Parent**: RULE-01
- **Type**: TechnicalSpec
"#;

    let doc = parse_document(content, dummy_path)
        .expect("All 6 specification layers with non-standard prefixes must parse cleanly");

    assert_eq!(doc.nodes.len(), 6);

    // Layer 1: Vision
    assert_eq!(doc.nodes[0].id().as_str(), "GOAL-01");
    assert_eq!(doc.nodes[0].layer(), Layer::Vision);
    assert_eq!(doc.nodes[0].layer().rank(), 1);
    assert!(doc.nodes[0].parents().is_empty());

    // Layer 2: Strategy
    assert_eq!(doc.nodes[1].id().as_str(), "ROAD-01");
    assert_eq!(doc.nodes[1].layer(), Layer::Strategy);
    assert_eq!(doc.nodes[1].layer().rank(), 2);
    assert_eq!(doc.nodes[1].parents()[0].as_str(), "GOAL-01");

    // Layer 3: Epic
    assert_eq!(doc.nodes[2].id().as_str(), "CAP-01");
    assert_eq!(doc.nodes[2].layer(), Layer::Epic);
    assert_eq!(doc.nodes[2].layer().rank(), 3);
    assert_eq!(doc.nodes[2].parents()[0].as_str(), "ROAD-01");

    // Layer 4: UserStory
    assert_eq!(doc.nodes[3].id().as_str(), "PERSONA-01");
    assert_eq!(doc.nodes[3].layer(), Layer::UserStory);
    assert_eq!(doc.nodes[3].layer().rank(), 4);
    assert_eq!(doc.nodes[3].parents()[0].as_str(), "CAP-01");

    // Layer 5: Requirement
    assert_eq!(doc.nodes[4].id().as_str(), "RULE-01");
    assert_eq!(doc.nodes[4].layer(), Layer::Requirement);
    assert_eq!(doc.nodes[4].layer().rank(), 5);
    assert_eq!(doc.nodes[4].parents()[0].as_str(), "PERSONA-01");

    // Layer 6: TechnicalSpec
    assert_eq!(doc.nodes[5].id().as_str(), "IMPL-01");
    assert_eq!(doc.nodes[5].layer(), Layer::TechnicalSpec);
    assert_eq!(doc.nodes[5].layer().rank(), 6);
    assert_eq!(doc.nodes[5].parents()[0].as_str(), "RULE-01");
}

/// Exhaustively verifies all documented string representations, aliases, and casing variants
/// for each specification layer when specified via `- **Type**:`.
///
/// // @trace SPEC-FMT-FRONTMATTER, SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_regression_parser_explicit_type_layer_variations_and_aliases() {
    let dummy_path = Path::new("docs/variations.md");

    let test_cases: Vec<(&str, Layer)> = vec![
        // Layer 1: Vision
        ("Vision", Layer::Vision),
        ("vision", Layer::Vision),
        ("VISION", Layer::Vision),
        ("  Vision  ", Layer::Vision),
        // Layer 2: Strategy
        ("Strategy", Layer::Strategy),
        ("strategy", Layer::Strategy),
        ("STRATEGY", Layer::Strategy),
        ("  Strategy  ", Layer::Strategy),
        // Layer 3: Epic
        ("Epic", Layer::Epic),
        ("epic", Layer::Epic),
        ("EPIC", Layer::Epic),
        ("  Epic  ", Layer::Epic),
        // Layer 4: UserStory
        ("UserStory", Layer::UserStory),
        ("userstory", Layer::UserStory),
        ("USERSTORY", Layer::UserStory),
        ("User Story", Layer::UserStory),
        ("user story", Layer::UserStory),
        ("Story", Layer::UserStory),
        ("story", Layer::UserStory),
        // Layer 5: Requirement
        ("Requirement", Layer::Requirement),
        ("requirement", Layer::Requirement),
        ("REQUIREMENT", Layer::Requirement),
        ("Functional Requirement", Layer::Requirement),
        ("functional requirement", Layer::Requirement),
        ("Non-Functional Requirement", Layer::Requirement),
        ("non-functional requirement", Layer::Requirement),
        ("NFR", Layer::Requirement),
        ("nfr", Layer::Requirement),
        // Layer 6: TechnicalSpec
        ("TechnicalSpec", Layer::TechnicalSpec),
        ("technicalspec", Layer::TechnicalSpec),
        ("TECHNICALSPEC", Layer::TechnicalSpec),
        ("Technical Spec", Layer::TechnicalSpec),
        ("technical spec", Layer::TechnicalSpec),
        ("Technical Specification", Layer::TechnicalSpec),
        ("technical specification", Layer::TechnicalSpec),
        ("Specification", Layer::TechnicalSpec),
        ("specification", Layer::TechnicalSpec),
        ("Spec", Layer::TechnicalSpec),
        ("spec", Layer::TechnicalSpec),
    ];

    for (type_str, expected_layer) in test_cases {
        let content = format!(
            r#"---
document_type: custom_spec
id: CUST-SPEC-001
title: Custom Spec
version: 1.0.0
status: approved
---

### CUSTOM-01: Custom Node
- **ID**: CUSTOM-01
- **Parent**: None
- **Type**: {}
"#,
            type_str
        );

        let doc = parse_document(&content, dummy_path)
            .unwrap_or_else(|err| panic!("Failed to parse - **Type**: '{}': {:?}", type_str, err));

        assert_eq!(
            doc.nodes[0].layer(),
            expected_layer,
            "Failed matching layer for - **Type**: '{}'",
            type_str
        );
    }
}

/// Verifies that invalid type names or leaf layers (SourceCode, AutomatedTest)
/// specified in specification documents fail with `ParseError::GrammarViolation` and exit code 2.
///
/// // @trace SPEC-FMT-FRONTMATTER, SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_regression_parser_invalid_or_disallowed_explicit_types() {
    let dummy_path = Path::new("docs/invalid_types.md");

    let invalid_types = [
        "NonExistentLayer",
        "Database",
        "SourceCode",    // Leaf layer not allowed in spec documents
        "AutomatedTest", // Leaf layer not allowed in spec documents
        "12345",
    ];

    for invalid_type in invalid_types {
        let content = format!(
            r#"---
document_type: custom_spec
id: CUST-SPEC-001
title: Custom Spec
version: 1.0.0
status: approved
---

### CUSTOM-02: Invalid Type Node
- **ID**: CUSTOM-02
- **Parent**: None
- **Type**: {}
"#,
            invalid_type
        );

        let err = parse_document(&content, dummy_path).unwrap_err();
        assert_eq!(
            err.exit_code(),
            2,
            "Expected exit code 2 for invalid explicit type: '{}'",
            invalid_type
        );
        assert!(
            matches!(err, ParseError::GrammarViolation { .. }),
            "Expected GrammarViolation for invalid explicit type: '{}', got {:?}",
            invalid_type,
            err
        );
    }
}

#[test]
fn test_parser_collects_multiple_grammar_errors_in_document() {
    let dummy_path = Path::new("docs/multi_error.md");
    let content = r#"---
document_type: custom_spec
id: MULTI-SPEC-001
title: Multi Error Spec
version: 1.0.0
status: approved
---

### REQ_BAD_UNDERSCORE: First Bad Node
- **ID**: REQ_BAD_UNDERSCORE
- **Parent**: None
- **Type**: Requirement

### REQ-BAD-PARENT: Second Bad Node
- **ID**: REQ-BAD-PARENT
- **Parent**: BAD_PARENT_UNDERSCORE
- **Type**: Requirement

### REQ-MISMATCH: Third Bad Node
- **ID**: REQ-MISMATCH-DIFFERENT
- **Parent**: None
- **Type**: Requirement
"#;

    let err = parse_document(content, dummy_path).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert_eq!(err.errors().len(), 3);
    let err_str = err.to_string();
    assert!(err_str.contains("Multiple parse errors:"));
}
