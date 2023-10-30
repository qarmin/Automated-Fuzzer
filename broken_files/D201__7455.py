def maybe_async(callable_, *args, **kwargs):\

    """
    Turn a callable into a coroutine if it isn't
    """
    return asyncio.coroutine(callable_)(*args, **kwargs)