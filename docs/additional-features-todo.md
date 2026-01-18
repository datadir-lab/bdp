# bdp Development Roadmap (Release v1.0)

## 1. Core CLI & Granular Pulls 🧬
- [ ] `bdp init`: Initialize project with `bdp.json` and `.gitignore` defaults.
- [ ] **Support Macro/Micro Sources**:
    - [ ] `bdp source add uniprot:swissprot`: Pull entire curated datasets.
    - [ ] `bdp source add uniprot:p01234`: Pull specific protein/entry (Subsetting).
- [ ] **Recursive Dependencies**: Allow a data source to depend on another (e.g., `genome` depends on `annotations`).
- [ ] **Bundles Support**:
    - [ ] `bdp source add bundle:human-standard`: Pull a pre-verified set of compatible genome, proteome, and index files.
- [ ] `bdp sync`: Atomic command for "Pull + Verify + Post-Pull + Export".

## 2. Post-Pull & Plugin System 🔌
- [ ] Implement `--post <action>` flag for automated processing.
- [ ] **Built-in Recipe Library**:
    - [ ] `index`: Auto-run `samtools faidx`, `tabix`, or `bwa index`.
    - [ ] `blast`: Auto-run `makeblastdb`.
- [ ] **HPC Optimization**: Implement symlinking for shared caches to save storage quota.

## 3. Scientific Reporting & Paper Automation 📄
- [ ] `bdp report --format das`: Generate a **Data Availability Statement** for journal submission.
- [ ] `bdp report --format methods`: Generate a LaTeX/Markdown snippet of the technical data environment.
- [ ] `bdp cite`: 
    - [ ] Auto-lookup **RRIDs** (Research Resource Identifiers) for tools/data.
    - [ ] Export `references.bib` with correct versions/DOIs.
- [ ] `bdp bundle --supplement`: Package `bdp.json`, `audit_trail.json`, and citations into a `.zip` for reviewers.

## 4. Audit & Provenance 📜
- [ ] `bdp audit`: Generate `audit_trail.json` (The "Black Box" recorder).
    - [ ] Log: Origin URL, Local Path, Timestamp, SHA-256, and Hook Exit Codes.
- [ ] `bdp verify`: Check all local files against the manifest hashes.

## 5. Infrastructure (EU/Sovereignty) 🇪🇺
- [ ] **S3 Mirroring**: Automate ingest of UniProt/NCBI into OVHcloud (EU-based).
- [ ] **Metadata Registry**: Host the central "Recipe" registry in a sovereign EU data center.
- [ ] **Static Binaries**: Compile for Linux/macOS to ensure zero-dependency execution on HPC.