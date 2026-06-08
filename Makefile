CRATES := mdd mdd-usecase mdd-dfd mdd-tree mdd-er mdd-sequence mdd-state mdd-infra mdd-gantt mdd-flowchart mdd-swimlane mdd-grid mdd-analysis mdd-steps mdd-ranking mdd-group-multi mdd-layer mdd-timeline mdd-before-after mdd-cycle mdd-process mdd-funnel mdd-pyramid mdd-triangle mdd-matrix mdd-compare mdd-scale mdd-swot mdd-venn mdd-radial mdd-concept mdd-mindmap mdd-puzzle mdd-group mdd-table mdd-list-v mdd-list-h mdd-list-grid mdd-kpi mdd-map mdd-math mdd-todo mdd-persona mdd-tweet mdd-slack mdd-kanban mdd-radar mdd-pie mdd-journey mdd-wireframe mdd-changelog mdd-faq mdd-quote mdd-pricetable mdd-org mdd-gitgraph mdd-dirtree mdd-github mdd-demo

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
