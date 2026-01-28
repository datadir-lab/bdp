# RefGenie Application Note - Paper Analysis

## Paper Citation

**Title:** Refgenie: a reference genome resource manager

**Journal:** GigaScience, Volume 9, Issue 2, February 2020

**DOI:** 10.1093/gigascience/giz149

**PMID:** 31995185

**PMC:** PMC6988606

**Authors:** Michal Stolarczyk, Vincent P. Reuter, Jason P. Smith, Neal E. Magee, Nathan C. Sheffield

**Affiliations:** Center for Public Health Genomics, Department of Biochemistry and Molecular Genetics, Research Computing, Department of Public Health Sciences, and Department of Biomedical Engineering, University of Virginia, Charlottesville, VA, USA

---

## 1. Full Title

**"Refgenie: a reference genome resource manager"**

---

## 2. Complete Abstract (4 Parts)

### Background
Reference genome assemblies are essential for high-throughput sequencing analysis projects. Typically, genome assemblies are stored on disk alongside related resources; e.g., many sequence aligners require the assembly to be indexed. The resulting indexes are broadly applicable for downstream analysis, so it makes sense to share them. However, there is no simple tool to do this.

### Results
Here, we introduce refgenie, a reference genome assembly asset manager. Refgenie makes it easier to organize, retrieve, and share genome analysis resources. In addition to genome indexes, refgenie can manage any files related to reference genomes, including sequences and annotation files. Refgenie includes a command line interface and a server application that provides a RESTful API, so it is useful for both tool development and analysis.

### Conclusions
Refgenie streamlines sharing genome analysis resources among groups and across computing environments.

### Availability
Refgenie is available at https://refgenie.databio.org. An archival copy of the code is available via the GigaScience database, GigaDB.

---

## 3. Section Headings and Approximate Word Counts

| Section | Estimated Word Count | Description |
|---------|---------------------|-------------|
| **Abstract** | ~150 words | Structured abstract with Background, Results, Conclusions, Availability |
| **Background/Introduction** | ~300-400 words | Problem statement and motivation |
| **Implementation** | ~800-1000 words | Technical details of software architecture |
| **Results/Findings** | ~500-600 words | Key capabilities and comparisons |
| **Conclusions** | ~100-150 words | Summary and future directions |
| **Availability of Source Code and Requirements** | ~100 words | License, requirements, repository info |
| **Acknowledgements** | ~50 words | Funding and contributions |
| **References** | 27 references | Bibliography |

**Total Estimated Length:** ~2,000-2,500 words (typical for GigaScience Application Notes)

---

## 4. Implementation Section Structure

The Implementation section is organized into the following subsections:

### 4.1 Overview: Build vs Pull
- Refgenie provides two ways to obtain genome assets: **pull** (download pre-built) and **build** (create locally)
- For common assets, pulling pre-built, remote-hosted assets obviates the need to install and run specialized software
- For uncommon assets or disconnected computers, users can build assets locally

### 4.2 Software Architecture (Tripartite Design)
Refgenie consists of three independent packages:

1. **refgenconf** - Configuration/API package
   - Python API to interact with genome configuration files
   - Intended for programmatic use
   - Used by both CLI and server packages
   - Third-party Python packages can leverage this

2. **refgenie CLI** - Command Line Interface
   - Main user-facing tool installed via `pip install refgenie`
   - Commands: `refgenie pull`, `refgenie build`, `refgenie seek`, `refgenie list`, `refgenie init`

3. **refgenieserver** - Server Application
   - Provides web interface and RESTful API
   - Built using FastAPI and uvicorn
   - Runs in Docker container with mounted assets
   - Enables remote asset distribution

### 4.3 Genome Configuration File
- YAML format file that tracks metadata and local file paths
- Stores paths to individual genome assembly resources ("assets")
- Each asset represents one or more files tied to a particular genome assembly
- Think of assets as folders of related files

### 4.4 Asset Building
- Uses locally available software (e.g., bowtie2-build) or containerized software via Docker flag
- Lists requirements for each asset type
- Produces standardized output for any genome assembly

---

## 5. Figures and Tables

### Figure 1: Refgenie Concept and Software Organization
**Panel A:** Build vs Pull workflow diagram
- Shows that refgenie provides ability to either build or pull assets
- Build: Uses local or containerized tools to create assets
- Pull: Downloads pre-built assets from remote server

**Panel B:** Genome Configuration File
- Shows YAML format structure
- Demonstrates how refgenie reads/writes configuration
- Tracks available local assets

**Panel C:** Software Architecture
- Illustrates tripartite structure: conf utility, CLI, server package
- Shows how components interact
- Users and software access via CLI or web API

### Figure 2: Server Software Stack (Inferred)
- Docker container architecture diagram
- Shows how archived assets are mounted
- Illustrates FastAPI/uvicorn stack
- Web and API user interfaces

### Table 1: Assets Available for Build

| Asset Type | Build Time (H:MM) | Memory (GB) | Disk Size (GB) |
|------------|-------------------|-------------|----------------|
| fasta | - | - | - |
| bowtie2_index | - | - | - |
| bwa_index | - | - | - |
| hisat2_index | - | - | - |
| salmon_index | - | - | - |
| star_index | - | - | - |
| bismark_bt2_index | - | - | - |
| (and others) | - | - | - |

*Note: Assets built for human genome using single core. Times and memory are representative values from single run.*

### Table 2: Feature Comparison

| Feature | iGenomes | Galaxy Data Managers | genomepy | Refgenie |
|---------|----------|---------------------|----------|----------|
| Pre-built downloads | Bulk only | Yes* | Yes | Individual |
| Build assets | No | Yes | No | Yes |
| Custom genomes | No | Yes | No | Yes |
| Remote API | No | No | No | Yes |
| CLI | No | No | Yes | Yes |
| Python API | No | No | Yes | Yes |
| Standalone | Yes | No | Yes | Yes |

*Data managers' assets can be accessed individually but not outside of Galaxy user interface

---

## 6. Reference Count and Style

### Reference Count: 27 references

### Citation Style
GigaScience uses numbered references in Vancouver/NLM style:
- In-text citations: [1], [2], [8,9,14-17]
- Reference list: Numbered consecutively in order of first appearance

### Key Referenced Works (Examples)
1. iGenomes project
2. Galaxy Data Managers
3. genomepy tool
4. Bowtie2 aligner
5. BWA aligner
6. HISAT2 aligner
7. Salmon quantification tool
8. STAR aligner
9. Bismark bisulfite aligner
10. Supporting data in GigaDB (Reference 27)

---

## 7. Code Examples

### Installation
```bash
pip install refgenie
```

### Initialize Configuration
```bash
refgenie init -c refgenie.yaml -s http://rg.databio.org
```

### Pull Pre-built Assets
```bash
refgenie pull hg38/bwa_index
refgenie pull hg38/bowtie2_index
```

### Build Assets Locally
```bash
refgenie build custom_genome/bowtie2_index
refgenie build rCRSd/fasta -c refgenie.yaml --files fasta=rCRSd.fa.gz -R
```

### Seek Asset Path (for pipeline integration)
```bash
refgenie seek hg38/salmon_index
```

### List Available Assets
```bash
refgenie list      # Local assets
refgenie listr     # Remote assets
```

### Check Build Requirements
```bash
refgenie build --requirements hg38/bowtie2_index
```

### Build with Docker
```bash
refgenie build hg38/bowtie2_index -d  # Uses containerized software
```

### Python API Usage (refgenconf)
```python
import refgenconf

# Load configuration
rgc = refgenconf.RefGenConf("refgenie.yaml")

# Get asset path
path = rgc.seek("hg38", "bowtie2_index")

# List available genomes
genomes = rgc.list_genomes_by_asset("fasta")
```

---

## 8. Key Takeaways for Application Note Structure

### What Makes This a Good Application Note

1. **Concise:** ~2,000-2,500 words total
2. **Structured abstract:** Clear Background/Results/Conclusions/Availability
3. **Problem-solution framing:** States problem clearly before introducing solution
4. **Figures are informative:** Conceptual diagrams, not just screenshots
5. **Comparison table:** Positions tool against alternatives
6. **Practical examples:** Shows actual command-line usage
7. **Reproducibility:** GigaDB archival copy, clear installation instructions
8. **Modular implementation:** Well-organized codebase description

### GigaScience Application Note Format

- **Length:** Short (2,000-3,000 words typical)
- **Figures:** 1-4 figures, conceptual rather than results-heavy
- **Tables:** Feature comparison, performance metrics
- **Code:** Command-line examples, not extensive code blocks
- **Focus:** Tool description, not extensive benchmarking
- **Availability:** Strong emphasis on open source, reproducibility

---

## Sources

- [GigaScience Article](https://academic.oup.com/gigascience/article/9/2/giz149/5717403)
- [PubMed](https://pubmed.ncbi.nlm.nih.gov/31995185/)
- [PMC Full Text](https://pmc.ncbi.nlm.nih.gov/articles/PMC6988606/)
- [ResearchGate PDF](https://www.researchgate.net/publication/338908112_Refgenie_a_reference_genome_resource_manager)
- [Refgenie Documentation](https://refgenie.databio.org)
- [Refgenie GitHub](https://github.com/refgenie/refgenie)
