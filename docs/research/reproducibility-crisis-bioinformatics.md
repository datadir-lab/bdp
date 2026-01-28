# The Reproducibility Crisis in Bioinformatics: A Comprehensive Review

## Executive Summary

The reproducibility crisis in computational biology and bioinformatics represents one of the most significant challenges facing modern biomedical research. This document compiles academic evidence demonstrating that reproducibility rates in computational life sciences hover around 20% or less, with cascading effects on patient safety, research funding, and scientific progress.

---

## 1. Landmark Reproducibility Studies

### 1.1 The 2009 Ioannidis Microarray Study (11% Reproducibility)

**Citation:** Ioannidis, J.P.A., Allison, D.B., Ball, C.A., Coulibaly, I., Cui, X., Culhane, A.C., Falchi, M., Furlanello, C., et al. (2009). Repeatability of published microarray gene expression analyses. *Nature Genetics*, 41(2), 149-155. https://doi.org/10.1038/ng.295

**Key Findings:**
- Evaluated replication of data analyses in 18 articles on microarray-based gene expression profiling published in *Nature Genetics* in 2005-2006
- Only 2 of 18 analyses could be reproduced in principle (11%)
- Six were partially reproducible with some discrepancies
- Ten could not be reproduced at all
- Main reason for failure: data unavailability
- Discrepancies mostly due to incomplete data annotation or specification of data processing and analysis

**Impact:** This study was foundational in establishing that reproducibility in bioinformatics was severely limited, predating the broader "reproducibility crisis" awareness.

---

### 1.2 NLM Reproducibility Workshops (2018-2019)

**Citation:** Zaringhalam, M. & Federer, L. (2020). Data and Code for Reproducible Research: Lessons Learned from the NLM Reproducibility Workshop. Zenodo. https://doi.org/10.5281/zenodo.3818329

**Workshop Details:**
- The National Library of Medicine held two hands-on workshops in May 2019 (May 15-17, 2019)
- NIH intramural researchers attempted to reproduce five bioinformatics studies
- Location: 6001 Executive Boulevard, Bethesda, MD

**Key Findings:**
- **Zero of five studies could be fully reproduced**
- Not a single one of the ten teams over two workshops could fully reproduce the results of any paper
- Primary barriers: missing data, unavailable software, and inadequate documentation
- Despite papers claiming public data and code availability, practical reproduction failed

**Workshop Resources:**
- GitHub repository with lecture slides from Burke Squires' opening lecture on open science
- Resources for Jupyter notebooks, Anaconda, version control, Git, GitHub
- Presentations on containerization and Docker

---

### 1.3 Jupyter Notebook Reproducibility Studies

#### 1.3.1 Large-Scale GitHub Study (4% Reproducibility)

**Citation:** Pimentel, J.F., Murta, L., Braganholo, V., & Freire, J. (2019). A Large-Scale Study About Quality and Reproducibility of Jupyter Notebooks. *Proceedings of the 16th International Conference on Mining Software Repositories (MSR 2019)*, 507-517. https://doi.org/10.1109/MSR.2019.00077

**Key Findings:**
- Analyzed 1.4 million notebooks from GitHub
- Of 863,878 attempted executions of valid notebooks:
  - Only 24.11% executed without errors
  - **Only 4.03% produced the same results**
- 29.23% of notebook executions failed due to ImportError and ModuleNotFoundError exceptions
- These exceptions are related to missing dependencies

#### 1.3.2 Biomedical Publications Study (5.9% Reproducibility)

**Citation:** Samuel, S. & Konig-Ries, B. (2024). Computational reproducibility of Jupyter notebooks from biomedical publications. *GigaScience*, 13, giad113. https://doi.org/10.1093/gigascience/giad113

**Key Findings:**
- Analyzed 27,271 notebooks from 2,660 GitHub repositories associated with 3,467 articles
- 22,578 notebooks were written in Python
- 15,817 had dependencies declared in standard requirement files
- For 10,388 of these, all declared dependencies could be installed
- Of these, 1,203 notebooks ran without errors
- **Only 879 (approximately 5.9%) produced results identical to those reported**
- 9,100 (87.6%) notebooks resulted in exceptions

---

### 1.4 Harvard Dataverse R Scripts Study (26% Success Rate)

**Citation:** Trisovic, A., Lau, M.K., Pasquier, T., & Crosas, M. (2022). A large-scale study on research code quality and execution. *Scientific Data*, 9, 60. https://doi.org/10.1038/s41597-022-01143-6

**Key Findings:**
- Retrieved and analyzed more than 2,000 replication datasets with over 9,000 unique R files published from 2010 to 2020
- 74% of R files failed to complete without error in initial execution
- **Only 26% of scripts completed without errors**
- With automatic code cleaning applied: 56% still failed
- Most common errors: missing library, incorrect working directory, and missing file errors

---

## 2. The Duke University Chemotherapy Scandal

### 2.1 Overview

The Duke University scandal involving Anil Potti represents one of the most significant research misconduct cases in computational biology, with direct patient harm implications.

### 2.2 The Original Research

**Retracted Paper:** Potti, A., Dressman, H.K., Bild, A., et al. (2006). Genomic signatures to guide the use of chemotherapeutics. *Nature Medicine*, 12(11), 1294-1300. https://doi.org/10.1038/nm1491 (RETRACTED)

**Claims:**
- Developed gene expression signatures that predict sensitivity to individual chemotherapeutic drugs
- Claimed signatures could accurately predict clinical response in individuals treated with these drugs
- Cited 252 times before retraction

### 2.3 The Investigation

**Key Paper:** Baggerly, K.A. & Coombes, K.R. (2009). Deriving chemosensitivity from cell lines: Forensic bioinformatics and reproducible research in high-throughput biology. *The Annals of Applied Statistics*, 3(4), 1309-1334. https://doi.org/10.1214/09-AOAS291

**Investigators:** Keith Baggerly and Kevin Coombes, MD Anderson Cancer Center, University of Texas

**Problems Discovered:**
- Off-by-one indexing errors
- Label reversals (sensitive/resistant cell lines switched)
- Data inconsistencies and duplications
- Genes listed that were not actually on the microarrays being analyzed
- Data sets appeared to be altered to make drug response predictors look more accurate

**Challenges in Reporting:**
- Took 150-200 days of forensic analysis
- Only provided partial, changing data
- Cancer journals rejected their findings as "too negative"
- Eventually published in *Annals of Applied Statistics*

### 2.4 Clinical Impact

**Timeline:**
- 2006: Original papers published in Nature Medicine, NEJM, and other journals
- 2007: Clinical trials launched at Duke and Moffitt Cancer Center despite ongoing concerns
- 2007-2010: Patients allocated to treatment arms based on flawed results
- 2010: Trials terminated, Potti suspended and resigned
- 2015: Office of Research Integrity found Potti guilty of research misconduct

**Patient Harm:**
- Patients potentially given wrong chemotherapy drugs
- Duke served eight lawsuits by families of affected patients
- Lawsuits alleged exposure to harmful and unnecessary chemotherapy
- Cases settled out of court
- Duke reimbursed American Cancer Society nearly $730,000 in grant money

### 2.5 Papers Retracted

As of 2024, Potti has had 11 research publications retracted, including papers in:
- Nature Medicine (2006)
- Journal of Clinical Oncology (2007)
- Lancet Oncology (2007)
- Journal of the American Medical Association (2008)
- PLoS One (2008)
- Proceedings of the National Academy of Sciences (2008)
- New England Journal of Medicine (2006)

**Official Finding:** NOT-OD-16-021: Findings of Research Misconduct (Federal Register, November 9, 2015)

---

## 3. Excel Gene Name Errors

### 3.1 Original Discovery (2016)

**Citation:** Ziemann, M., Eren, Y., & El-Osta, A. (2016). Gene name errors are widespread in the scientific literature. *Genome Biology*, 17, 177. https://doi.org/10.1186/s13059-016-1044-7

**Key Findings:**
- Microsoft Excel auto-converts gene names to dates and floating-point numbers
- Examples: SEPT2 (Septin 2) → "2-Sep"; MARCH1 → "1-Mar"
- Approximately one-fifth (19.6%) of papers with supplementary Excel gene lists contain erroneous conversions
- 987 supplementary files from 704 articles confirmed affected
- Journals with highest proportion affected (>20%): Nucleic Acids Research, Genome Biology, Nature Genetics, Genome Research, Genes and Development, Nature
- Positive correlation between journal impact factor and proportion of affected lists
- Linear-regression showed errors increasing at 15% annually (vs 3.8% increase in papers)
- **Conversions are irreversible** - original gene names cannot be recovered
- At least 30 gene names affected by date conversions; 2,000+ affected if Riken identifiers included

### 3.2 Follow-up Study (2021)

**Citation:** Abeysooriya, M., Soria, M., Kasu, M.S., & Ziemann, M. (2021). Gene name errors: Lessons not learned. *PLoS Computational Biology*, 17(7), e1008984. https://doi.org/10.1371/journal.pcbi.1008984

**Key Findings:**
- Gene name errors continued to accumulate unabated after 2016
- Improved scanning identified errors in **30.9% (3,436/11,117)** of articles with supplementary Excel gene lists
- This is significantly higher than the 2016 estimate
- Despite Human Gene Name Consortium changing susceptible gene names, problem persists
- Errors also include conversion to internal date format (five-digit numbers)
- Demonstrates spreadsheets are ill-suited for large genomic data

---

## 4. Other Major Reproducibility Failures

### 4.1 Industry Drug Target Validation Studies

#### Amgen Study (11% Reproducibility)

**Citation:** Begley, C.G. & Ellis, L.M. (2012). Raise standards for preclinical cancer research. *Nature*, 483, 531-533. https://doi.org/10.1038/483531a

**Key Findings:**
- Scientists at Amgen tried to reproduce 53 high-profile cancer studies
- **Succeeded with only 6 (approximately 11%)**
- 47 of 53 studies could not be replicated

#### Bayer Study (25% Reproducibility)

**Citation:** Prinz, F., Schlange, T., & Asadullah, K. (2011). Believe it or not: how much can we rely on published data on potential drug targets? *Nature Reviews Drug Discovery*, 10, 712. https://doi.org/10.1038/nrd3439-c1

**Key Findings:**
- Bayer reviewed 67 internal drug discovery projects based on literature reports
- Results matched published findings in only 14 projects
- **Only 20-25% of projects had reproducible results**
- In-house experimental data did not match literature claims in 65% of projects

### 4.2 Nature Survey on Reproducibility Crisis

**Citation:** Baker, M. (2016). 1,500 scientists lift the lid on reproducibility. *Nature*, 533, 452-454. https://doi.org/10.1038/533452a

**Key Findings:**
- Survey of 1,576 researchers
- **More than 70% tried and failed to reproduce another scientist's experiments**
- **More than 50% failed to reproduce their own experiments**
- 52% agreed there is a significant "crisis" of reproducibility
- 90% acknowledged a reproducibility crisis exists

### 4.3 BioModels Repository Study (51% Reproducibility)

**Citation:** Tiwari, K., Kananathan, S., Roberts, M.G., et al. (2021). Reproducibility in systems biology modelling. *Molecular Systems Biology*, 17, e20209982. https://doi.org/10.15252/msb.20209982

**Key Findings:**
- Analyzed 455 mathematical models from 152 peer-reviewed journals
- Represented work of approximately 1,400 scientists from 49 countries
- **Only 51% (233/455) of models directly reproduced simulation results**
- 49% could not be reproduced using manuscript information
- Additional 12% reproducible with empirical correction or author support
- 37% remained non-reproducible due to missing parameter values, initial concentrations, or inconsistent model structure
- Proposed 8-point reproducibility scorecard for journals

### 4.4 Reproducibility Project: Cancer Biology (26% Completion)

**Citation:** Errington, T.M., Denis, A., Perfito, N., Iorns, E., & Nosek, B.A. (2021). Reproducibility in Cancer Biology: What have we learned? *eLife*, 10, e75830. https://doi.org/10.7554/eLife.75830

**Related Papers:**
- Errington, T.M., et al. (2021). Investigating the replicability of preclinical cancer biology. *eLife*, 10, e71601. https://doi.org/10.7554/eLife.71601
- Errington, T.M., et al. (2021). Experiments from unfinished Registered Reports in the Reproducibility Project: Cancer Biology. *eLife*, 10, e73430. https://doi.org/10.7554/eLife.73430

**Key Findings (8-year project, 2012-2021):**
- Planned to replicate 193 experiments from 53 high-profile papers (2010-2012)
- **Only 50 experiments (26%) completed** due to reproducibility challenges at every phase
- 27% of original experiments presented only representative images
- 21% reported statistical tests without specifying which test
- Raw data publicly accessible for only 2% of experiments
- Requests for data sharing mostly failed

---

## 5. Database Version and Annotation Problems

### 5.1 Reference Sequence Database Issues

**Citation:** Various authors (2024). Ten common issues with reference sequence databases and how to mitigate them. *Frontiers in Bioinformatics*, 4, 1278228. https://doi.org/10.3389/fbinf.2024.1278228

**Key Issues:**
- **Taxonomic misannotation**: Incorrect taxonomic identity assigned to sequences
- Results in false positive/negative taxa detections
- T2T human genome contains ~200 million additional base pairs over GRCh38
- Failure to use updated reference genomes impacts metagenomic results

### 5.2 Pathway Analysis Software Annotation Errors

**Citation:** Various authors (2010). Pathway Analysis Software: Annotation Errors and Solutions. *Molecular & Cellular Proteomics*, PMC2950253.

**Example of Impact:**
- In March 2008: Glucocorticoid receptor signaling ranked 5th in pathway analysis
- In September 2008: Same data, same analysis → ranked 27th
- Results affected by variation in gene symbol annotations across software releases
- Input ID type analyzed affects results

### 5.3 Enzyme Commission Number Errors

**Citation:** Various authors (2005). Genome annotation errors in pathway databases due to semantic ambiguity in partial EC numbers. *BMC Bioinformatics*, PMC1179732.

**Key Findings:**
- Systematic annotation errors from misinterpretation of partial EC numbers (e.g., '1.1.1.-')
- Databases like KEGG used for training bioinformatics algorithms
- Algorithms trained on faulty data learn incorrect rules and parameters
- Errors propagate through the research ecosystem

### 5.4 Genome Annotation Quality Challenges

**Key Issues:**
- Genome annotation produces considerable errors and "ridiculous identifications"
- Different annotation procedures and numerous databases create spectrum of quality
- New information continuously emerges, requiring constant re-annotation
- No standard means to recompute annotations with latest software/databases
- Large portion of stored data may be incorrect or incomplete

---

## 6. The Five Pillars of Computational Reproducibility

**Citation:** Ziemann, M., Poulain, P., & Bora, A. (2023). The five pillars of computational reproducibility: bioinformatics and beyond. *Briefings in Bioinformatics*, 24(6), bbad375. https://doi.org/10.1093/bib/bbad375

**The Five Pillars:**
1. **Literate Programming** - Combining code with narrative documentation
2. **Code Version Control and Sharing** - Git, GitHub, public repositories
3. **Compute Environment Control** - Docker, containers, Guix for reproducible environments
4. **Persistent Data Sharing** - Public repositories with DOIs
5. **Documentation** - Complete methods, parameters, and workflow descriptions

**Key Recommendations:**
- Docker containers for environment reproducibility (small performance reduction)
- Guix for bit-for-bit build reproducibility and verifiability
- Note: Docker images are not guaranteed to build reproducibly in future due to link decay

---

## 7. Estimated Reproducibility Rates Summary

| Study/Domain | Year | Reproducibility Rate | Citation |
|-------------|------|---------------------|----------|
| Microarray gene expression | 2009 | 11% | Ioannidis et al., Nature Genetics |
| NLM Bioinformatics Workshop | 2019 | 0% (0/5 studies) | Zaringhalam & Federer |
| Jupyter notebooks (GitHub) | 2019 | 4% | Pimentel et al., MSR |
| Jupyter notebooks (biomedical) | 2024 | 5.9% | Samuel & Konig-Ries, GigaScience |
| R scripts (Harvard Dataverse) | 2022 | 26% | Trisovic et al., Scientific Data |
| Amgen cancer studies | 2012 | 11% | Begley & Ellis, Nature |
| Bayer drug targets | 2011 | 25% | Prinz et al., Nature Rev Drug Discov |
| BioModels mathematical models | 2021 | 51% | Tiwari et al., Mol Syst Biol |
| Excel gene name errors | 2021 | 69% error-free | Abeysooriya et al., PLoS Comp Biol |

---

## 8. Consequences of Irreproducibility

### 8.1 Patient Safety
- Duke University: Patients potentially given incorrect chemotherapy based on flawed computational predictions
- Clinical decisions increasingly rely on genomic/proteomic data
- Misdiagnoses when sequencing data quality is compromised
- Patients may receive ineffective treatments or miss beneficial ones

### 8.2 Research Impact
- Misleads the scientific community
- Wastes research funding
- Slows scientific progress
- Erodes public confidence in science
- Tarnishes reputation of institutions and colleagues

### 8.3 Economic Impact
- Bayer: 65% of target-validation projects discontinued due to non-reproducible literature claims
- Duke reimbursed $730,000 to American Cancer Society
- Pharmaceutical industry estimates billions lost on non-reproducible research

---

## 9. Root Causes

### 9.1 Technical Factors
- Missing dependencies and broken links
- Software version incompatibilities
- Database updates changing results
- Stochastic algorithms introducing variability
- Technical variability from different sequencing platforms

### 9.2 Documentation Failures
- Incomplete method descriptions
- Missing parameter specifications
- Unclear data processing steps
- Representative images without quantitative data

### 9.3 Data Availability
- Raw data rarely publicly accessible (2% in cancer biology studies)
- Data sharing requests often fail
- Incomplete or changing datasets provided

### 9.4 Cultural Factors
- Pressure to publish quickly
- Disincentives to spend resources on replication
- Career damage for those who report failures
- "Too negative" findings rejected by journals

### 9.5 Training Gaps
- Bioinformaticians often trained in biology OR computer science, not both
- Limited formal training in software development best practices
- Academic budgets don't support professional development teams

---

## 10. Recommendations for Researchers

### 10.1 For Authors
1. Use literate programming (Jupyter, R Markdown)
2. Version control all code (Git/GitHub)
3. Use containers (Docker) for environment specification
4. Share data in public repositories with DOIs
5. Document all parameters, software versions, and database versions
6. Avoid spreadsheet software for genomic data
7. Test that your own analysis can be reproduced from scratch

### 10.2 For Journals
1. Require public data deposition
2. Require code availability
3. Implement reproducibility checklists
4. Adopt the 8-point reproducibility scorecard for computational models
5. Accept replication studies and negative results

### 10.3 For Institutions
1. Provide reproducibility training
2. Reward reproducible research practices
3. Support replication studies
4. Investigate concerns promptly (lesson from Duke)

---

## References (Alphabetical by First Author)

1. Abeysooriya, M., Soria, M., Kasu, M.S., & Ziemann, M. (2021). Gene name errors: Lessons not learned. *PLoS Computational Biology*, 17(7), e1008984. https://doi.org/10.1371/journal.pcbi.1008984

2. Baggerly, K.A. & Coombes, K.R. (2009). Deriving chemosensitivity from cell lines: Forensic bioinformatics and reproducible research in high-throughput biology. *The Annals of Applied Statistics*, 3(4), 1309-1334. https://doi.org/10.1214/09-AOAS291

3. Baker, M. (2016). 1,500 scientists lift the lid on reproducibility. *Nature*, 533, 452-454. https://doi.org/10.1038/533452a

4. Begley, C.G. & Ellis, L.M. (2012). Raise standards for preclinical cancer research. *Nature*, 483, 531-533. https://doi.org/10.1038/483531a

5. Errington, T.M., Denis, A., Perfito, N., Iorns, E., & Nosek, B.A. (2021). Reproducibility in Cancer Biology: What have we learned? *eLife*, 10, e75830. https://doi.org/10.7554/eLife.75830

6. Errington, T.M., et al. (2021). Investigating the replicability of preclinical cancer biology. *eLife*, 10, e71601. https://doi.org/10.7554/eLife.71601

7. Ioannidis, J.P.A., Allison, D.B., Ball, C.A., et al. (2009). Repeatability of published microarray gene expression analyses. *Nature Genetics*, 41(2), 149-155. https://doi.org/10.1038/ng.295

8. Pimentel, J.F., Murta, L., Braganholo, V., & Freire, J. (2019). A Large-Scale Study About Quality and Reproducibility of Jupyter Notebooks. *Proceedings of MSR 2019*, 507-517. https://doi.org/10.1109/MSR.2019.00077

9. Potti, A., et al. (2006). Genomic signatures to guide the use of chemotherapeutics. *Nature Medicine*, 12(11), 1294-1300. https://doi.org/10.1038/nm1491 (RETRACTED)

10. Prinz, F., Schlange, T., & Asadullah, K. (2011). Believe it or not: how much can we rely on published data on potential drug targets? *Nature Reviews Drug Discovery*, 10, 712. https://doi.org/10.1038/nrd3439-c1

11. Samuel, S. & Konig-Ries, B. (2024). Computational reproducibility of Jupyter notebooks from biomedical publications. *GigaScience*, 13, giad113. https://doi.org/10.1093/gigascience/giad113

12. Tiwari, K., Kananathan, S., Roberts, M.G., et al. (2021). Reproducibility in systems biology modelling. *Molecular Systems Biology*, 17, e20209982. https://doi.org/10.15252/msb.20209982

13. Trisovic, A., Lau, M.K., Pasquier, T., & Crosas, M. (2022). A large-scale study on research code quality and execution. *Scientific Data*, 9, 60. https://doi.org/10.1038/s41597-022-01143-6

14. Zaringhalam, M. & Federer, L. (2020). Data and Code for Reproducible Research: Lessons Learned from the NLM Reproducibility Workshop. Zenodo. https://doi.org/10.5281/zenodo.3818329

15. Ziemann, M., Eren, Y., & El-Osta, A. (2016). Gene name errors are widespread in the scientific literature. *Genome Biology*, 17, 177. https://doi.org/10.1186/s13059-016-1044-7

16. Ziemann, M., Poulain, P., & Bora, A. (2023). The five pillars of computational reproducibility: bioinformatics and beyond. *Briefings in Bioinformatics*, 24(6), bbad375. https://doi.org/10.1093/bib/bbad375

---

## Appendix: Official Documents

### Office of Research Integrity Findings
- NOT-OD-16-021: Findings of Research Misconduct (Federal Register, November 9, 2015)
- Available at: https://grants.nih.gov/grants/guide/notice-files/NOT-OD-16-021.html

### NLM Workshop Materials
- Workshop Website: https://nlm-repro.github.io/
- Short Report: https://www.nlm.nih.gov/od/osi/documents/Short_Report_on_Reproducibility_Workshop.pdf

---

*Document compiled: January 2026*
*Sources verified via web search and academic databases*
