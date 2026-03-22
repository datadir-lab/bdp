import typer

app = typer.Typer(name="bdp-embed", help="BDP embedding pipeline CLI")

# Subcommands registered in each module
# NOTE: embed, project, and tiles modules will be created in Tasks 4, 5, and 6
from bdp_embed import embed  # noqa: E402, F401
from bdp_embed import project  # noqa: E402, F401

if __name__ == "__main__":
    app()
