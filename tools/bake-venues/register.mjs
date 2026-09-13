// SPDX-License-Identifier: GPL-3.0-or-later
/** Loader bootstrap: `node --import tools/bake-venues/register.mjs …` */
import { register } from "node:module";

register("./resolve-hooks.mjs", import.meta.url);
