/** One web resource read from `resources/community/sites.json`. */
export interface ResourceSite {
  id: string;
  title: string;
  url: string;
  description: string;
  categories: string[];
  tags: string[];
  featured: boolean;
  /** Local `data:image/...` icon, or null when the file is missing. */
  icon: string | null;
}

export interface ResourceCategory {
  id: string;
  title: string;
}

export interface ResourceCatalog {
  version: number;
  updatedAt: string;
  categories: ResourceCategory[];
  sites: ResourceSite[];
}
