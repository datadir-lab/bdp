import typer

app = typer.Typer(name="bdp-embed", help="BDP embedding pipeline CLI")

# Subcommands registered in each module
# NOTE: embed, project, and tiles modules will be created in Tasks 4, 5, and 6
# For now, these imports are commented out to allow tests to run
# from bdp_embed import embed, project, tiles  # noqa: E402, F401

if __name__ == "__main__":
    app()
