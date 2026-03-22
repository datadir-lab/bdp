import json
from bdp_embed.tiles import build_quadtree, get_tile_key, points_in_bounds


def make_point(x, y, i=0):
    return {"id": str(i), "x": x, "y": y, "l": f"P{i}", "et": "data_source",
            "st": "protein", "org": "uniprot", "slug": f"p{i}"}


def test_points_in_bounds_filters_correctly():
    pts = [make_point(1.0, 1.0), make_point(5.0, 5.0), make_point(-1.0, -1.0)]
    result = points_in_bounds(pts, x_min=0, x_max=3, y_min=0, y_max=3)
    assert len(result) == 1
    assert result[0]["x"] == 1.0


def test_get_tile_key_format():
    key = get_tile_key("abc123", z=3, tx=2, ty=1)
    assert key == "vectors/tiles/abc123/3/2/1.json"


def test_build_quadtree_returns_nonempty_tiles():
    pts = [make_point(float(i % 10), float(i // 10), i) for i in range(100)]
    tiles = build_quadtree(pts, run_id="test", zoom_min=0, zoom_max=3)
    # At least one tile at zoom 0
    assert any(t["z"] == 0 for t in tiles)
    # All tile keys end in .json
    assert all(t["key"].endswith(".json") for t in tiles)


def test_build_quadtree_coarse_tiles_have_fewer_points():
    pts = [make_point(float(i % 10), float(i // 10), i) for i in range(1000)]
    tiles = build_quadtree(pts, run_id="test", zoom_min=0, zoom_max=5)
    zoom0_tiles = [t for t in tiles if t["z"] == 0]
    zoom5_tiles = [t for t in tiles if t["z"] == 5]
    zoom0_count = sum(len(t["points"]) for t in zoom0_tiles)
    zoom5_count = sum(len(t["points"]) for t in zoom5_tiles)
    assert zoom0_count <= zoom5_count
