.PHONY: test pbt fuzzing fuzzing-list check clippy fmt clean

# 全テストを実行する
test:
	cargo test

# Property-Based Testing を実行する
pbt:
	cargo test --test 'proptest_*'

# Fuzzing を全ターゲットで 30 秒ずつ実行する
fuzzing:
	@for target in $$(cargo fuzz list); do \
		echo "=== Fuzzing $$target ==="; \
		cargo +nightly fuzz run $$target -- -max_total_time=30 || exit 1; \
	done

# Fuzzing ターゲット一覧を表示する
fuzzing-list:
	cargo fuzz list

# cargo check を実行する
check:
	cargo check

# cargo clippy を実行する
clippy:
	cargo clippy

# cargo fmt を実行する
fmt:
	cargo fmt

# ビルド成果物を削除する
clean:
	cargo clean
