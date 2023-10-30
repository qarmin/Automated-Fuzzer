import pytest
@pytest.mark.parametrize(
    [
        (  # ts1 - load 2 dgs, extract 2 tables with interpolation
        (  # ts3 - load & extract, multiple transforms
        ),
        ),
    ],
)
def test_run_with_plot(inpath, tname, outpath, outfig, datadir):
    dg, tab = dgpost.run(inpath)