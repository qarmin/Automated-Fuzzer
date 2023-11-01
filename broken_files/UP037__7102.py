from __future__ import annotations
class IdentityLinearOperator(ConstantDiagLinearOperator):
    def _mul_matrix(
        other: Union[Float[torch.Tensor, "... #M #N"], Float[LinearOperator, "... #M #N"]],
    ) -> Float[LinearOperator, "... M N"]:
        return other
