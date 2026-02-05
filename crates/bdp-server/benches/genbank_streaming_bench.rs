// GenBank streaming decompression benchmarks
//
// Compares streaming vs non-streaming decompression for:
// - Throughput (records/sec)
// - Memory usage
// - Decompression speed

use bdp_server::ingest::genbank::models::SourceDatabase;
use bdp_server::ingest::genbank::parser::GenbankParser;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::{Cursor, Write};

fn create_compressed_data(repeat: usize) -> Vec<u8> {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let sample_data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Create larger data by repeating
    let mut large_data = String::new();
    for i in 0..repeat {
        let modified = sample_data.replace("NC_001416", &format!("NC_{:06}", i));
        large_data.push_str(&modified);
    }

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(large_data.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

fn bench_nonstreaming_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_nonstreaming");

    for size in [1, 10, 50, 100].iter() {
        let compressed = create_compressed_data(*size);
        let data_size = compressed.len();

        group.throughput(Throughput::Bytes(data_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let mut decoder = flate2::read::GzDecoder::new(cursor);
                    let mut decompressed = Vec::new();
                    std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();

                    let parser = GenbankParser::new(SourceDatabase::Genbank);
                    let records = parser.parse_all(decompressed.as_slice()).unwrap();
                    black_box(records)
                });
            },
        );
    }

    group.finish();
}

fn bench_streaming_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_streaming");

    for size in [1, 10, 50, 100].iter() {
        let compressed = create_compressed_data(*size);
        let data_size = compressed.len();

        group.throughput(Throughput::Bytes(data_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let decoder = flate2::read::GzDecoder::new(cursor);

                    let parser = GenbankParser::new(SourceDatabase::Genbank);
                    let records = parser.parse_all(decoder).unwrap();
                    black_box(records)
                });
            },
        );
    }

    group.finish();
}

fn bench_parse_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_only");

    for size in [1, 10, 50, 100].iter() {
        let compressed = create_compressed_data(*size);

        // Decompress once
        let cursor = Cursor::new(compressed);
        let mut decoder = flate2::read::GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        let data_size = decompressed.len();

        group.throughput(Throughput::Bytes(data_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x", size)),
            &decompressed,
            |b, data| {
                b.iter(|| {
                    let parser = GenbankParser::new(SourceDatabase::Genbank);
                    let records = parser.parse_all(data.as_slice()).unwrap();
                    black_box(records)
                });
            },
        );
    }

    group.finish();
}

fn bench_decompress_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompress_only");

    for size in [1, 10, 50, 100].iter() {
        let compressed = create_compressed_data(*size);
        let data_size = compressed.len();

        group.throughput(Throughput::Bytes(data_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x_nonstreaming", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let mut decoder = flate2::read::GzDecoder::new(cursor);
                    let mut decompressed = Vec::new();
                    std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
                    black_box(decompressed)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x_streaming", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let decoder = flate2::read::GzDecoder::new(cursor);
                    // Simulate streaming read
                    let mut buffer = Vec::new();
                    let mut reader = std::io::BufReader::new(decoder);
                    std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();
                    black_box(buffer)
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    let compressed = create_compressed_data(100);

    // Non-streaming: decompress all at once
    group.bench_function("nonstreaming_all_at_once", |b| {
        b.iter(|| {
            let cursor = Cursor::new(compressed.clone());
            let mut decoder = flate2::read::GzDecoder::new(cursor);
            let mut decompressed = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
            black_box(decompressed)
        });
    });

    // Streaming: read in chunks
    group.bench_function("streaming_chunked_read", |b| {
        b.iter(|| {
            let cursor = Cursor::new(compressed.clone());
            let mut decoder = flate2::read::GzDecoder::new(cursor);
            let mut total = 0;
            let mut buffer = [0u8; 8192]; // 8KB chunks

            loop {
                let bytes_read = std::io::Read::read(&mut decoder, &mut buffer).unwrap();
                if bytes_read == 0 {
                    break;
                }
                total += bytes_read;
            }
            black_box(total)
        });
    });

    group.finish();
}

fn bench_compression_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_levels");

    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let sample_data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    for (name, level) in [
        ("fast", Compression::fast()),
        ("default", Compression::default()),
        ("best", Compression::best()),
    ]
    .iter()
    {
        // Compress with this level
        let mut encoder = GzEncoder::new(Vec::new(), *level);
        encoder.write_all(sample_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(name), &compressed, |b, data| {
            b.iter(|| {
                let cursor = Cursor::new(data.clone());
                let decoder = flate2::read::GzDecoder::new(cursor);
                let parser = GenbankParser::new(SourceDatabase::Genbank);
                let records = parser.parse_all(decoder).unwrap();
                black_box(records)
            });
        });
    }

    group.finish();
}

fn bench_throughput_records_per_second(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_records_per_sec");

    for size in [10, 50, 100].iter() {
        let compressed = create_compressed_data(*size);

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_records_nonstreaming", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let mut decoder = flate2::read::GzDecoder::new(cursor);
                    let mut decompressed = Vec::new();
                    std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
                    let parser = GenbankParser::new(SourceDatabase::Genbank);
                    let records = parser.parse_all(decompressed.as_slice()).unwrap();
                    black_box(records)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_records_streaming", size)),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(data.clone());
                    let decoder = flate2::read::GzDecoder::new(cursor);
                    let parser = GenbankParser::new(SourceDatabase::Genbank);
                    let records = parser.parse_all(decoder).unwrap();
                    black_box(records)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_nonstreaming_decompression,
    bench_streaming_decompression,
    bench_parse_only,
    bench_decompress_only,
    bench_memory_patterns,
    bench_compression_levels,
    bench_throughput_records_per_second,
);

criterion_main!(benches);
