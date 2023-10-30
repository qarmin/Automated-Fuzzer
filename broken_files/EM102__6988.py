class Dependency:
    def validate(self) -> bool:
                raise ValueError(f"""Must provide a value for argument '{self.argument}' if {self.mode.__name__} of: {", ".join(f"'{arg}'" for arg in self.arguments)} are truthy.""")
