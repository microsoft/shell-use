from __future__ import annotations

from typing import Any, Dict, Iterable, List, Mapping, Optional, Tuple, Union

from .errors import make_error

EnvLike = Union[Mapping[str, str], Iterable[Tuple[str, str]], None]


def unwrap(resp: Dict[str, Any]) -> Any:
    if resp.get("ok"):
        return resp.get("data")
    raise make_error(resp.get("kind"), resp.get("message") or "shell-use error")


def env_pairs(env: EnvLike) -> List[List[str]]:
    if env is None:
        return []
    items = env.items() if isinstance(env, Mapping) else env
    return [[str(k), str(v)] for k, v in items]
