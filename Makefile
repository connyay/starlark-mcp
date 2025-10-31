format.fix:
	@echo "Formatting Starlark files..."
	buildifier -mode=fix -v -r extensions/
	@echo "Formatting complete."

format.check:
	@echo "Checking Starlark file formatting..."
	buildifier -mode=check -r extensions/
	@echo "Format check complete."

format.lint:
	@echo "Linting and fixing Starlark files..."
	buildifier -mode=fix -lint=fix -v -r extensions/
	@echo "Linting complete."
