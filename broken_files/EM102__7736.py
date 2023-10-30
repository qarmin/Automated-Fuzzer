class ModelMeta(DeclarativeMeta):
                raise AttributeError(f"""Attribute(s) {", ".join([f"'{dupe}'" for dupe in intersection])} was/were provided twice.""")