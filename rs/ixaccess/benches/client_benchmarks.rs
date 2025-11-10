use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ixaccess::IxAccessClient;
use std::sync::Arc;

// Helper function to create a temporary file path for benchmarks
fn temp_bench_path(name: &str) -> String {
    format!("/tmp/ixaccess_bench_{}.ixaccess", name)
}

// Helper to clean up test files
fn cleanup_path(path: &str) {
    let _ = std::fs::remove_file(path);
}

// Setup helper: Create a flat role structure (no hierarchy)
async fn setup_flat_structure(client: &IxAccessClient, num_roles: usize) {
    let roles: Vec<String> = (0..num_roles).map(|i| format!("role_{}", i)).collect();
    client.add_roles(roles).await.unwrap();
}

// Setup helper: Create a linear hierarchy (role_0 -> role_1 -> role_2 -> ...)
async fn setup_linear_hierarchy(client: &IxAccessClient, depth: usize) {
    let roles: Vec<String> = (0..depth).map(|i| format!("role_{}", i)).collect();
    client.add_roles(roles.clone()).await.unwrap();

    // Create linear chain: role_i is assigned role_{i+1}
    for i in 0..depth - 1 {
        client
            .assign_role(&format!("role_{}", i), &format!("role_{}", i + 1))
            .await
            .unwrap();
    }
}

// Setup helper: Create a tree hierarchy (each role has N children)
async fn setup_tree_hierarchy(client: &IxAccessClient, depth: usize, branching_factor: usize) {
    let mut roles = vec!["root".to_string()];
    client.add_role("root").await.unwrap();

    for level in 0..depth {
        let level_start = roles.len();
        let level_size = branching_factor.pow(level as u32);

        for i in 0..level_size {
            let role_name = format!("role_l{}_n{}", level, i);
            roles.push(role_name.clone());
            client.add_role(&role_name).await.unwrap();

            // Assign to parent
            if level == 0 {
                client.assign_role("root", &role_name).await.unwrap();
            } else {
                let parent_idx =
                    level_start - branching_factor.pow((level - 1) as u32) + (i / branching_factor);
                client
                    .assign_role(&roles[parent_idx], &role_name)
                    .await
                    .unwrap();
            }
        }
    }
}

// Setup helper: Create a diamond pattern (multiple inheritance paths)
async fn setup_diamond_hierarchy(client: &IxAccessClient, width: usize) {
    client.add_role("top").await.unwrap();

    // Create middle layer
    for i in 0..width {
        let middle_role = format!("middle_{}", i);
        client.add_role(&middle_role).await.unwrap();
        client.assign_role(&middle_role, "top").await.unwrap();
    }

    // Create bottom role that inherits from all middle roles
    client.add_role("bottom").await.unwrap();
    for i in 0..width {
        client
            .assign_role("bottom", &format!("middle_{}", i))
            .await
            .unwrap();
    }
}

// Setup helper: Add resources to roles
async fn setup_resources(client: &IxAccessClient, roles: Vec<&str>, resources_per_role: usize) {
    for role in roles {
        for i in 0..resources_per_role {
            client
                .assign_resource_to_role(role, "resource", &format!("value_{}_{}", role, i))
                .await
                .unwrap();
        }
    }
}

// Benchmark: list_roles with varying dataset sizes
fn bench_list_roles_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_roles_varying_sizes");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let path = temp_bench_path(&format!("list_roles_{}", size));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_flat_structure(&client, *size).await;
            Arc::new(client)
        });

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = Arc::clone(&client);
                async move {
                    let roles = client.list_roles().await.unwrap();
                    black_box(roles);
                }
            });
        });

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: list_all_roles_for_role with linear hierarchy (different depths)
fn bench_list_all_roles_linear_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_all_roles_linear_hierarchy");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for depth in [5, 10, 20, 50].iter() {
        group.throughput(Throughput::Elements(*depth as u64));

        let path = temp_bench_path(&format!("linear_hierarchy_{}", depth));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_linear_hierarchy(&client, *depth).await;
            Arc::new(client)
        });

        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = Arc::clone(&client);
                async move {
                    // Query the root role (should traverse entire chain)
                    let roles = client.list_all_roles_for_role("role_0").await.unwrap();
                    black_box(roles);
                }
            });
        });

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: list_all_roles_for_role with tree hierarchy (different branching factors)
fn bench_list_all_roles_tree_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_all_roles_tree_hierarchy");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for (depth, branching) in [(3, 2), (3, 4), (4, 3), (5, 2)].iter() {
        let total_roles = ((*branching as usize).pow(*depth as u32) - 1) / (branching - 1);
        group.throughput(Throughput::Elements(total_roles as u64));

        let path = temp_bench_path(&format!("tree_d{}_b{}", depth, branching));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_tree_hierarchy(&client, *depth, *branching).await;
            Arc::new(client)
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth{}_branch{}", depth, branching)),
            &(depth, branching),
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let client = Arc::clone(&client);
                    async move {
                        // Query root (should traverse entire tree)
                        let roles = client.list_all_roles_for_role("root").await.unwrap();
                        black_box(roles);
                    }
                });
            },
        );

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: list_all_roles_for_role with diamond pattern (multiple inheritance)
fn bench_list_all_roles_diamond_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_all_roles_diamond_pattern");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for width in [2, 5, 10, 20].iter() {
        group.throughput(Throughput::Elements(*width as u64));

        let path = temp_bench_path(&format!("diamond_{}", width));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_diamond_hierarchy(&client, *width).await;
            Arc::new(client)
        });

        group.bench_with_input(BenchmarkId::from_parameter(width), width, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = Arc::clone(&client);
                async move {
                    // Query bottom role (should traverse diamond and deduplicate)
                    let roles = client.list_all_roles_for_role("bottom").await.unwrap();
                    black_box(roles);
                }
            });
        });

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: get_all_resources_for_role_by_tag with varying resource counts
fn bench_get_resources_varying_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_resources_varying_counts");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for resource_count in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*resource_count as u64));

        let path = temp_bench_path(&format!("resources_{}", resource_count));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            client.add_role("test_role").await.unwrap();
            setup_resources(&client, vec!["test_role"], *resource_count).await;
            Arc::new(client)
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(resource_count),
            resource_count,
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let client = Arc::clone(&client);
                    async move {
                        let resources = client
                            .get_all_resources_for_role_by_tag("test_role", "resource")
                            .await
                            .unwrap();
                        black_box(resources);
                    }
                });
            },
        );

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: get_all_resources_for_role_by_tag with inherited resources
fn bench_get_resources_with_inheritance(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_resources_with_inheritance");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for depth in [2, 5, 10, 20].iter() {
        let resources_per_role = 10;
        group.throughput(Throughput::Elements((depth * resources_per_role) as u64));

        let path = temp_bench_path(&format!("resources_inherited_{}", depth));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_linear_hierarchy(&client, *depth).await;

            // Add resources to each role in the chain
            for i in 0..*depth {
                setup_resources(&client, vec![&format!("role_{}", i)], resources_per_role).await;
            }

            Arc::new(client)
        });

        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = Arc::clone(&client);
                async move {
                    // Query root role (should collect resources from entire chain)
                    let resources = client
                        .get_all_resources_for_role_by_tag("role_0", "resource")
                        .await
                        .unwrap();
                    black_box(resources);
                }
            });
        });

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: get_all_resources_for_role_by_tag with diamond pattern (deduplication)
fn bench_get_resources_diamond_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_resources_diamond_deduplication");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for width in [2, 5, 10, 20].iter() {
        let resources_per_role = 10;
        group.throughput(Throughput::Elements(
            *width as u64 * resources_per_role as u64,
        ));

        let path = temp_bench_path(&format!("resources_diamond_{}", width));
        cleanup_path(&path);

        let client = rt.block_on(async {
            let client = IxAccessClient::new(&path).await;
            setup_diamond_hierarchy(&client, *width).await;

            // Add same resources to top (will be inherited through multiple paths)
            setup_resources(&client, vec!["top"], resources_per_role).await;

            // Add unique resources to middle roles
            for i in 0..*width {
                setup_resources(&client, vec![&format!("middle_{}", i)], resources_per_role).await;
            }

            Arc::new(client)
        });

        group.bench_with_input(BenchmarkId::from_parameter(width), width, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = Arc::clone(&client);
                async move {
                    // Query bottom role (should deduplicate resources from multiple paths)
                    let resources = client
                        .get_all_resources_for_role_by_tag("bottom", "resource")
                        .await
                        .unwrap();
                    black_box(resources);
                }
            });
        });

        cleanup_path(&path);
    }
    group.finish();
}

// Benchmark: Query performance at different hierarchy levels
fn bench_query_at_different_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_at_different_levels");
    let rt = tokio::runtime::Runtime::new().unwrap();

    let depth = 10;
    let path = temp_bench_path("hierarchy_levels");
    cleanup_path(&path);

    let client = rt.block_on(async {
        let client = IxAccessClient::new(&path).await;
        setup_linear_hierarchy(&client, depth).await;
        Arc::new(client)
    });

    for level in [0, 2, 5, 9].iter() {
        let expected_roles = depth - level;
        group.throughput(Throughput::Elements(expected_roles as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("level_{}", level)),
            level,
            |b, &level| {
                b.to_async(&rt).iter(|| {
                    let client = Arc::clone(&client);
                    async move {
                        let roles = client
                            .list_all_roles_for_role(&format!("role_{}", level))
                            .await
                            .unwrap();
                        black_box(roles);
                    }
                });
            },
        );
    }

    cleanup_path(&path);
    group.finish();
}

// Benchmark: Cache hit vs cache miss scenarios
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");
    let rt = tokio::runtime::Runtime::new().unwrap();

    let size = 1000;
    let path = temp_bench_path("cache_test");
    cleanup_path(&path);

    let client = rt.block_on(async {
        let client = IxAccessClient::new(&path).await;
        setup_flat_structure(&client, size).await;
        Arc::new(client)
    });

    // Warm up cache
    rt.block_on(async {
        let _ = client.list_roles().await.unwrap();
    });

    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| {
            let client = Arc::clone(&client);
            async move {
                // Should hit cache (no external modifications)
                let roles = client.list_roles().await.unwrap();
                black_box(roles);
            }
        });
    });

    cleanup_path(&path);
    group.finish();
}

criterion_group!(
    benches,
    bench_list_roles_varying_sizes,
    bench_list_all_roles_linear_hierarchy,
    bench_list_all_roles_tree_hierarchy,
    bench_list_all_roles_diamond_pattern,
    bench_get_resources_varying_counts,
    bench_get_resources_with_inheritance,
    bench_get_resources_diamond_deduplication,
    bench_query_at_different_levels,
    bench_cache_performance,
);

criterion_main!(benches);
