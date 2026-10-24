"""
Automated Opaque-Box E2E Acceptance Test Suite for xtrc Repository Bootstrap.

Validates all requirements and acceptance criteria from ORIGINAL_REQUEST.md:
- R1: Project Overview & README
- R2: Rust Dev Container Environment
- R3: Repository Governance & Issue-First Workflow (CONTRIBUTING.md & .github/ISSUE_TEMPLATE/*)
- R4: AI Agent Instructions (copilot-instructions.md & AGENTS.md)
- Repository Hygiene & Metadata Isolation
"""

import json
import os
import re
import subprocess
import sys
import unittest
from pathlib import Path

# Resolve repository root path
REPO_ROOT = Path(__file__).resolve().parent.parent


def parse_jsonc(text: str):
    """
    Parse JSON with comments (JSONC) and trailing commas.
    Strips single-line (//) and multi-line (/* */) comments while preserving
    string literals.
    """
    def remove_comments(source: str) -> str:
        pattern = r'(\".*?(?<!\\)\")|(/\*.*?\*/)|(//[^\r\n]*$)'
        regex = re.compile(pattern, re.MULTILINE | re.DOTALL)
        def replace(match):
            if match.group(1):
                return match.group(1)
            return ""
        return regex.sub(replace, source)

    cleaned = remove_comments(text)
    # Remove trailing commas before } or ]
    cleaned = re.sub(r',\s*([}\]])', r'\1', cleaned)
    return json.loads(cleaned)


def load_yaml_safely(file_path: Path):
    """
    Safely parse YAML file.
    Uses Ruby's standard library psych YAML engine if available (standard on macOS),
    or falls back to a clean pure-Python lightweight YAML structural validator.
    """
    # 1. Try Ruby's built-in YAML parser if available
    try:
        ruby_check = subprocess.run(
            ["ruby", "-e", "require 'yaml'; puts YAML.load_file(ARGV[0]).to_json", str(file_path)],
            capture_output=True,
            text=True,
            check=False
        )
        if ruby_check.returncode == 0 and ruby_check.stdout.strip():
            return json.loads(ruby_check.stdout)
    except Exception:
        pass

    # 2. Try pyyaml if installed
    try:
        import yaml
        with open(file_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f)
    except ImportError:
        pass

    # 3. Fallback: Pure Python basic YAML line scanner for syntax sanity
    content = file_path.read_text(encoding="utf-8")
    lines = content.splitlines()
    # Check for unquoted colon followed by space in scalar strings
    for idx, line in enumerate(lines, 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        # Check basic mapping syntax
        if ":" in stripped:
            key, _, val = stripped.partition(":")
            val_clean = val.strip()
            # If value has unquoted colon with following space, report potential syntax error
            if val_clean and not (val_clean.startswith('"') or val_clean.startswith("'")):
                if ": " in val_clean and not val_clean.startswith("|") and not val_clean.startswith(">"):
                    raise ValueError(f"Potential unquoted mapping in YAML at line {idx}: {line}")
    return {"_parsed": True, "raw": content}


class TestR1Readme(unittest.TestCase):
    """Acceptance tests for Requirement R1: Project Overview & README."""

    def setUp(self):
        self.readme_path = REPO_ROOT / "README.md"
        self.assertTrue(self.readme_path.is_file(), f"README.md not found at {self.readme_path}")
        self.content = self.readme_path.read_text(encoding="utf-8")

    def test_r1_01_readme_exists_and_non_empty(self):
        """README.md exists and has non-trivial content."""
        self.assertGreater(len(self.content.strip()), 200, "README.md is too short")

    def test_r1_02_verbatim_mission_quote(self):
        """README.md contains the exact verbatim mission quote requested in R1."""
        expected_quote = (
            "xtrc is a fast, plain-text traceability CLI written in Rust. "
            "It tracks relationships across your entire workflow, linking vision, strategy, "
            "epics, and stories directly to technical specs and code. It lives purely in "
            "version control, ditching heavy databases so you can verify project alignment "
            "instantly right from your terminal."
        )
        self.assertIn(
            expected_quote,
            self.content,
            "README.md does not contain the exact verbatim mission quote from ORIGINAL_REQUEST.md"
        )

    def test_r1_03_title_and_badges(self):
        """README.md starts with # xtrc title."""
        self.assertTrue(
            self.content.startswith("# xtrc") or "\n# xtrc" in self.content,
            "README.md must start with title '# xtrc'"
        )

    def test_r1_04_vision_section(self):
        """README.md has a dedicated Vision section explaining ALM problems vs VCS-native solution."""
        vision_match = re.search(r"##\s+(?:The\s+)?Vision", self.content, re.IGNORECASE)
        self.assertIsNotNone(vision_match, "README.md is missing a '## Vision' section")
        # Check key concepts within vision
        lower_content = self.content.lower()
        self.assertIn("database", lower_content)
        self.assertIn("traceability", lower_content)
        self.assertTrue("jira" in lower_content or "alm" in lower_content or "doors" in lower_content)

    def test_r1_05_core_concepts_section(self):
        """README.md has Core Concepts section covering plain-text, VCS-native, graph, and terminal checks."""
        concepts_match = re.search(r"##\s+Core\s+Concepts", self.content, re.IGNORECASE)
        self.assertIsNotNone(concepts_match, "README.md is missing a '## Core Concepts' section")

        lower = self.content.lower()
        self.assertIn("plain-text", lower, "Core concepts must explain plain-text traceability")
        self.assertTrue("vcs-native" in lower or "version control" in lower, "Must explain VCS-native architecture")
        self.assertTrue("graph" in lower or "dag" in lower, "Must describe relationship graph")
        self.assertTrue("bidirectional" in lower, "Must describe bidirectional traceability")

    def test_r1_06_planned_workflow_section(self):
        """README.md has Planned Workflow section covering the developer lifecycle."""
        workflow_match = re.search(r"##\s+Planned\s+Workflow", self.content, re.IGNORECASE)
        self.assertIsNotNone(workflow_match, "README.md is missing a '## Planned Workflow' section")

        lower = self.content.lower()
        self.assertTrue("intent" in lower, "Workflow must include intent capture")
        self.assertTrue("spec" in lower, "Workflow must include technical specifications")
        self.assertTrue("code" in lower, "Workflow must include code implementation/linking")
        self.assertTrue("verify" in lower or "check" in lower, "Workflow must include alignment verification")
        self.assertTrue("ci" in lower or "cd" in lower, "Workflow must include CI/CD gating")

    def test_r1_07_development_and_getting_started(self):
        """README.md has Getting Started / Development section mentioning Dev Container and cargo."""
        dev_match = re.search(r"##\s+(?:Development|Getting\s+Started)", self.content, re.IGNORECASE)
        self.assertIsNotNone(dev_match, "README.md is missing a Getting Started / Development section")

        self.assertIn(".devcontainer", self.content, "Getting started must reference Dev Container")
        self.assertIn("cargo build", self.content, "Getting started must mention cargo build")
        self.assertIn("cargo test", self.content, "Getting started must mention cargo test")

    def test_r1_08_license_section(self):
        """README.md has a License section referencing MIT license."""
        license_match = re.search(r"##\s+License", self.content, re.IGNORECASE)
        self.assertIsNotNone(license_match, "README.md is missing a '## License' section")
        self.assertIn("MIT", self.content, "README License section must reference MIT")

    def test_r1_09_relative_links_validity(self):
        """Relative links referenced in README.md must point to existing files."""
        # Find markdown links [text](path)
        link_matches = re.findall(r'\[([^\]]+)\]\(([^)]+)\)', self.content)
        for text, link in link_matches:
            # Check only relative local file links (not URLs, anchors, or mailto)
            if not link.startswith("http://") and not link.startswith("https://") and not link.startswith("#") and not link.startswith("mailto:"):
                target_path = (self.readme_path.parent / link).resolve()
                self.assertTrue(
                    target_path.exists(),
                    f"README.md references broken local link '{link}' (points to non-existent {target_path})"
                )


class TestR2Devcontainer(unittest.TestCase):
    """Acceptance tests for Requirement R2: Rust Dev Container Environment."""

    def setUp(self):
        self.devcontainer_dir = REPO_ROOT / ".devcontainer"
        self.config_path = self.devcontainer_dir / "devcontainer.json"
        self.assertTrue(
            self.config_path.is_file(),
            f".devcontainer/devcontainer.json not found at {self.config_path}"
        )
        self.raw_content = self.config_path.read_text(encoding="utf-8")
        try:
            self.config = parse_jsonc(self.raw_content)
        except Exception as e:
            self.fail(f".devcontainer/devcontainer.json is not valid JSON/JSONC: {e}")

    def test_r2_01_valid_jsonc(self):
        """devcontainer.json is syntactically valid JSON/JSONC."""
        self.assertIsInstance(self.config, dict, "Parsed devcontainer.json must be a JSON object")

    def test_r2_02_rust_image_specified(self):
        """devcontainer.json specifies mcr.microsoft.com/devcontainers/rust or equivalent."""
        image = self.config.get("image", "")
        dockerfile = self.config.get("build", {}).get("dockerfile", "")
        has_rust_image = "mcr.microsoft.com/devcontainers/rust" in image or "rust" in image.lower() or bool(dockerfile)
        self.assertTrue(
            has_rust_image,
            f"Expected Rust devcontainer image in devcontainer.json, found image: '{image}'"
        )

    def test_r2_03_required_extensions_installed(self):
        """devcontainer.json installs required extensions: rust-analyzer, even-better-toml, GitHub.copilot."""
        customizations = self.config.get("customizations", {})
        vscode = customizations.get("vscode", {})
        extensions = vscode.get("extensions", [])
        self.assertIsInstance(extensions, list, "customizations.vscode.extensions must be a list")

        required_extensions = [
            "rust-lang.rust-analyzer",
            "tamasfe.even-better-toml",
            "GitHub.copilot"
        ]
        for ext in required_extensions:
            self.assertIn(
                ext,
                extensions,
                f"Missing required VS Code extension '{ext}' in devcontainer.json"
            )

    def test_r2_04_editor_settings_configured(self):
        """devcontainer.json configures VS Code settings for Rust and formatting."""
        customizations = self.config.get("customizations", {})
        vscode = customizations.get("vscode", {})
        settings = vscode.get("settings", {})
        self.assertIsInstance(settings, dict, "customizations.vscode.settings must be an object")

        # rust-analyzer check command configured
        check_command = settings.get("rust-analyzer.check.command", "")
        self.assertIn(
            check_command,
            ["clippy", "check"],
            f"Expected rust-analyzer.check.command to be clippy or check, got '{check_command}'"
        )

    def test_r2_05_toolchain_and_utilities(self):
        """devcontainer.json ensures cargo, clippy, rustfmt, and rust-analyzer toolchain components."""
        post_create = self.config.get("postCreateCommand", "")
        features = self.config.get("features", {})
        
        # Verify postCreateCommand or features covers rustup components or cargo
        has_toolchain_setup = (
            "clippy" in post_create
            or "rustfmt" in post_create
            or "rust-analyzer" in post_create
            or "cargo" in post_create
            or len(features) > 0
        )
        self.assertTrue(
            has_toolchain_setup,
            "devcontainer.json should ensure rust toolchain components (clippy, rustfmt, rust-analyzer)"
        )


class TestR3GovernanceAndIssueTemplates(unittest.TestCase):
    """Acceptance tests for Requirement R3: Repository Governance & Issue-First Workflow."""

    def setUp(self):
        self.contributing_path = REPO_ROOT / "CONTRIBUTING.md"
        self.issue_template_dir = REPO_ROOT / ".github" / "ISSUE_TEMPLATE"

    def test_r3_01_contributing_file_exists(self):
        """CONTRIBUTING.md exists and is non-empty."""
        self.assertTrue(self.contributing_path.is_file(), f"CONTRIBUTING.md not found at {self.contributing_path}")
        content = self.contributing_path.read_text(encoding="utf-8")
        self.assertGreater(len(content.strip()), 300, "CONTRIBUTING.md is too short")

    def test_r3_02_three_core_rules_documented(self):
        """CONTRIBUTING.md documents the 3 core rules: Issue-First, Requirements-First, Intent-over-How."""
        content = self.contributing_path.read_text(encoding="utf-8").lower()

        # Rule 1: Issue-Driven Development
        self.assertTrue(
            "issue-driven" in content or "issue-first" in content or "rule 1" in content,
            "CONTRIBUTING.md must document Rule 1: Issue-Driven Development"
        )
        self.assertTrue(
            "issue" in content and ("before" in content or "every" in content),
            "Rule 1 must require issues before writing code"
        )

        # Rule 2: Requirements Before Design
        self.assertTrue(
            "requirements before design" in content or "requirements-first" in content or "rule 2" in content,
            "CONTRIBUTING.md must document Rule 2: Requirements Before Design"
        )

        # Rule 3: Intent Over Implementation
        self.assertTrue(
            "intent over implementation" in content or "intent-over-how" in content or "rule 3" in content,
            "CONTRIBUTING.md must document Rule 3: Intent Over Implementation"
        )

    def test_r3_03_contributor_workflow_and_rust_standards(self):
        """CONTRIBUTING.md covers contributor lifecycle, quality gates, and Rust standards."""
        content = self.contributing_path.read_text(encoding="utf-8")

        # Mandatory quality gate commands
        self.assertIn("cargo fmt", content, "CONTRIBUTING.md must document cargo fmt")
        self.assertIn("cargo clippy", content, "CONTRIBUTING.md must document cargo clippy")
        self.assertIn("cargo test", content, "CONTRIBUTING.md must document cargo test")

        # Rust error handling standards
        lower = content.lower()
        self.assertTrue(
            "unwrap" in lower or "expect" in lower,
            "CONTRIBUTING.md must document strict policy on naked unwrap/expect"
        )
        self.assertTrue(
            "thiserror" in lower or "anyhow" in lower,
            "CONTRIBUTING.md must prescribe thiserror/anyhow"
        )

    def test_r3_04_issue_template_dir_exists(self):
        """The .github/ISSUE_TEMPLATE directory exists."""
        self.assertTrue(
            self.issue_template_dir.is_dir(),
            f".github/ISSUE_TEMPLATE directory not found at {self.issue_template_dir}"
        )

    def test_r3_05_config_yml_disables_blank_issues(self):
        """config.yml exists, is valid YAML, and disables blank issues."""
        config_path = self.issue_template_dir / "config.yml"
        self.assertTrue(config_path.is_file(), f"config.yml not found at {config_path}")
        
        parsed = load_yaml_safely(config_path)
        raw_text = config_path.read_text(encoding="utf-8")
        self.assertTrue(
            re.search(r"blank_issues_enabled:\s*false", raw_text, re.IGNORECASE),
            "config.yml must set 'blank_issues_enabled: false'"
        )

    def test_r3_06_feature_proposal_template(self):
        """feature_proposal.yml exists, is valid YAML, and structures intent and requirements."""
        fp_path = self.issue_template_dir / "feature_proposal.yml"
        self.assertTrue(fp_path.is_file(), f"feature_proposal.yml not found at {fp_path}")
        
        raw_text = fp_path.read_text(encoding="utf-8")
        parsed = load_yaml_safely(fp_path)
        self.assertIsNotNone(parsed)

        # Check required semantic sections
        lower = raw_text.lower()
        self.assertIn("intent", lower, "feature_proposal.yml must contain an Intent section")
        self.assertIn("requirement", lower, "feature_proposal.yml must contain a Requirements section")
        self.assertIn("acceptance", lower, "feature_proposal.yml must contain Acceptance Criteria")

        # Check validations required: true
        self.assertIn("required: true", raw_text)

    def test_r3_07_bug_report_template(self):
        """bug_report.yml exists, is valid YAML, and structures expected intent vs actual behavior."""
        br_path = self.issue_template_dir / "bug_report.yml"
        self.assertTrue(br_path.is_file(), f"bug_report.yml not found at {br_path}")
        
        raw_text = br_path.read_text(encoding="utf-8")
        parsed = load_yaml_safely(br_path)
        self.assertIsNotNone(parsed)

        lower = raw_text.lower()
        self.assertTrue("intent" in lower or "expected" in lower, "bug_report.yml must capture expected intent")
        self.assertTrue("actual" in lower or "behavior" in lower, "bug_report.yml must capture actual behavior")
        self.assertTrue("repro" in lower or "step" in lower, "bug_report.yml must capture reproduction steps")
        self.assertIn("required: true", raw_text)

    def test_r3_08_task_template(self):
        """task.yml exists, is valid YAML, and structures motivation, requirements, and verification."""
        task_path = self.issue_template_dir / "task.yml"
        self.assertTrue(task_path.is_file(), f"task.yml not found at {task_path}")
        
        raw_text = task_path.read_text(encoding="utf-8")
        parsed = load_yaml_safely(task_path)
        self.assertIsNotNone(parsed)

        lower = raw_text.lower()
        self.assertTrue("motivation" in lower or "purpose" in lower, "task.yml must capture motivation")
        self.assertTrue("deliverable" in lower or "requirement" in lower, "task.yml must capture requirements")
        self.assertTrue("verification" in lower or "checklist" in lower, "task.yml must capture verification")
        self.assertIn("required: true", raw_text)

    def test_r3_09_all_issue_templates_syntactically_valid_yaml(self):
        """All YAML files under .github/ISSUE_TEMPLATE/ parse cleanly without syntax errors."""
        yml_files = list(self.issue_template_dir.glob("*.yml")) + list(self.issue_template_dir.glob("*.yaml"))
        self.assertGreaterEqual(len(yml_files), 4, "Expected at least 4 YAML files in .github/ISSUE_TEMPLATE/")

        for yml_file in yml_files:
            try:
                parsed = load_yaml_safely(yml_file)
                self.assertIsNotNone(parsed, f"Failed to parse {yml_file.name}")
            except Exception as e:
                self.fail(f"YAML syntax error in {yml_file}: {e}")


class TestR4AgentInstructions(unittest.TestCase):
    """Acceptance tests for Requirement R4: AI Agent Instructions."""

    def setUp(self):
        self.copilot_path = REPO_ROOT / ".github" / "copilot-instructions.md"
        self.agents_md_path = REPO_ROOT / "AGENTS.md"
        self.gemini_rules_path = REPO_ROOT / ".gemini" / "rules" / "repository-governance.md"

    def test_r4_01_copilot_instructions_exists(self):
        """copilot-instructions.md exists at .github/copilot-instructions.md."""
        self.assertTrue(
            self.copilot_path.is_file(),
            f".github/copilot-instructions.md not found at {self.copilot_path}"
        )
        content = self.copilot_path.read_text(encoding="utf-8")
        self.assertGreater(len(content.strip()), 200, "copilot-instructions.md is too short")

    def test_r4_02_copilot_instructions_content(self):
        """copilot-instructions.md guides Copilot on 3 core rules, issue context, and Rust standards."""
        content = self.copilot_path.read_text(encoding="utf-8").lower()

        # Project mission / identity
        self.assertIn("xtrc", content)
        self.assertIn("rust", content)

        # 3 Core Rules
        self.assertTrue("issue" in content, "Must guide on issue-first workflow")
        self.assertTrue("requirements" in content, "Must guide on requirements-first")
        self.assertTrue("intent" in content, "Must guide on intent over implementation")

        # Rust standards
        self.assertTrue("unwrap" in content or "expect" in content, "Must warn against naked unwrap/expect")
        self.assertTrue("clippy" in content, "Must require clippy-clean code")

    def test_r4_03_agents_md_location_and_existence(self):
        """AGENTS.md exists at repository root, and NEVER inside .agents/teamwork/."""
        self.assertTrue(self.agents_md_path.is_file(), f"AGENTS.md not found at root {self.agents_md_path}")
        
        # Verify strict metadata isolation: AGENTS.md must NOT be inside .agents/teamwork/
        violating_path = REPO_ROOT / ".agents" / "teamwork" / "AGENTS.md"
        self.assertFalse(
            violating_path.exists(),
            "Violation: AGENTS.md must not exist in .agents/teamwork/ (reserved for metadata)"
        )

    def test_r4_04_agents_md_content_and_quality_gates(self):
        """AGENTS.md guides autonomous agents on rules, mandatory quality gates, and anti-cheating."""
        content = self.agents_md_path.read_text(encoding="utf-8")
        lower = content.lower()

        # Persona & project
        self.assertIn("xtrc", lower)
        self.assertIn("traceability", lower)

        # 3 Core Rules
        self.assertTrue("issue-driven" in lower or "issue-first" in lower)
        self.assertTrue("requirements before design" in lower or "requirements-first" in lower)
        self.assertTrue("intent over implementation" in lower or "intent-over-how" in lower)

        # Mandatory quality gate commands
        self.assertIn("cargo fmt", content)
        self.assertIn("cargo clippy", content)
        self.assertIn("cargo test", content)

        # Anti-cheating & integrity mandate
        self.assertTrue(
            "integrity" in lower or "hardcod" in lower or "bypass" in lower or "facade" in lower,
            "AGENTS.md must include integrity mandate prohibiting cheating or bypassing quality checks"
        )

    def test_r4_05_gemini_rules_mirror(self):
        """Repository provides native Gemini/Antigravity rules in .gemini/rules/."""
        self.assertTrue(
            self.gemini_rules_path.is_file(),
            f"Expected Gemini rule mirror at {self.gemini_rules_path}"
        )
        content = self.gemini_rules_path.read_text(encoding="utf-8").lower()
        self.assertIn("xtrc", content)
        self.assertTrue("issue" in content)
        self.assertTrue("requirements" in content)
        self.assertTrue("intent" in content)


class TestRepositoryHygieneAndIntegration(unittest.TestCase):
    """Acceptance tests for cross-component integration and repository hygiene."""

    def test_hygiene_01_license_file(self):
        """LICENSE file exists and contains MIT License text."""
        license_path = REPO_ROOT / "LICENSE"
        self.assertTrue(license_path.is_file(), "LICENSE file must exist")
        content = license_path.read_text(encoding="utf-8")
        self.assertIn("MIT License", content)

    def test_hygiene_02_gitignore_configured(self):
        """gitignore exists and covers standard Rust targets and OS artifacts."""
        gitignore_path = REPO_ROOT / ".gitignore"
        self.assertTrue(gitignore_path.is_file(), ".gitignore must exist")
        content = gitignore_path.read_text(encoding="utf-8")
        self.assertIn("/target", content, ".gitignore must ignore /target")
        self.assertIn(".DS_Store", content, ".gitignore must ignore .DS_Store")

    def test_hygiene_03_metadata_isolation(self):
        """The .agents/teamwork/ directory must contain only metadata, never source code or tests."""
        teamwork_dir = REPO_ROOT / ".agents" / "teamwork"
        if not teamwork_dir.exists():
            return

        # Strictly enforce metadata isolation: only .md or .gitkeep allowed, and no rule collisions
        for root, dirs, files in os.walk(teamwork_dir):
            for file in files:
                ext = Path(file).suffix.lower()
                is_metadata = (ext == ".md") or (file == ".gitkeep")
                self.assertTrue(
                    is_metadata,
                    f"Illicit non-metadata file '{file}' (extension '{ext}') found in agent metadata directory '{root}'"
                )
                self.assertNotIn(
                    file.upper(),
                    {"AGENTS.MD", "GEMINI.MD"},
                    f"Rule file collision '{file}' found in '{root}'"
                )


if __name__ == "__main__":
    unittest.main()
