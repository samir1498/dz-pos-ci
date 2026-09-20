// The category list.

import { z } from "zod";

import type { CategoryDto } from "../generated/CategoryDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { categorySchema } from "../schemas/catalogue";

export function categoriesClient({ send }: Transport) {
  return {
    async listCategories(): Promise<CategoryDto[]> {
      return narrow(await send("/categories"), z.array(categorySchema), "category list");
    },
  };
}
