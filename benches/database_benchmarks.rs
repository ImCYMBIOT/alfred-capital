use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use tempfile::TempDir;

use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};

fn create_test_transfer(id: u64) -> ProcessedTransfer {
    ProcessedTransfer {
        block_number: 1000 + id,
        transaction_hash: format!("0x{:064x}", id),
        log_index: 0,
        from_address: format!("0x{:040x}", id),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        amount: format!("{}", (id + 1) * 1000000000000000000),
        timestamp: 1640995200 + id,
        direction: if id % 2 == 0 { TransferDirection::ToBinance } else { TransferDirection::FromBinance },
    }
}

fn bench_database_insert(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("bench.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    let mut group = c.benchmark_group("database_insert");
    
    for size in [1, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("single_insert", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    let transfer = create_test_transfer(i);
                    let _ = database.store_transfer_and_update_net_flow(black_box(&transfer));
                }
            });
        });
    }
    
    group.finish();
}

fn bench_database_query(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("bench_query.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Pre-populate database
    for i in 0..1000 {
        let transfer = create_test_transfer(i);
        database.store_transfer_and_update_net_flow(&transfer).expect("Failed to store transfer");
    }
    
    let mut group = c.benchmark_group("database_query");
    
    group.bench_function("net_flow_query", |b| {
        b.iter(|| {
            let _ = database.get_net_flow_data();
        });
    });
    
    group.bench_function("transaction_count_query", |b| {
        b.iter(|| {
            let _ = database.get_transaction_count();
        });
    });
    
    group.bench_function("transaction_lookup", |b| {
        b.iter(|| {
            let tx_hash = format!("0x{:064x}", black_box(500));
            let _ = database.get_transaction(&tx_hash, 0);
        });
    });
    
    group.finish();
}

fn bench_net_flow_calculation(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("bench_calc.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    let mut group = c.benchmark_group("net_flow_calculation");
    
    for transfer_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("cumulative_calculation", transfer_count),
            transfer_count,
            |b, &transfer_count| {
                b.iter(|| {
                    // Clear database
                    let temp_dir = TempDir::new().expect("Failed to create temp directory");
                    let db_path = temp_dir.path().join("bench_temp.db");
                    let db = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
                    
                    for i in 0..transfer_count {
                        let transfer = create_test_transfer(i);
                        let _ = db.store_transfer_and_update_net_flow(black_box(&transfer));
                    }
                    
                    let _ = db.get_net_flow_data();
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets = bench_database_insert, bench_database_query, bench_net_flow_calculation
);
criterion_main!(benches);