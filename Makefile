CRATES := mdd mdd-usecase mdd-dfd mdd-tree mdd-er mdd-sequence mdd-state mdd-infra mdd-gantt mdd-flowchart mdd-swimlane mdd-grid mdd-analysis mdd-steps mdd-ranking mdd-group-multi mdd-layer

.PHONY: build test install uninstall clean

build:
	cargo build --release

test:
	cargo test

install:
	@for crate in $(CRATES); do \
		cargo install --path crates/$$crate; \
	done

uninstall:
	@for crate in $(CRATES); do \
		cargo uninstall $$crate 2>/dev/null || true; \
	done

clean:
	cargo clean
