'use client';

import * as React from 'react';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { useTranslations } from 'next-intl';
import { X as CloseIcon, Loader2, Search } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { SearchFilters as SearchFiltersType } from '@/lib/types/search';
import { apiClient } from '@/lib/api-client';
import { useDebounce } from '@/hooks/use-debounce';
import { listOrganizations } from '@/lib/api/organizations';
import type { OrganizationListItem } from '@/lib/types/organization';

interface SearchFiltersProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  filters: SearchFiltersType;
  onFiltersChange: (filters: SearchFiltersType) => void;
}

const FILTER_OPTIONS = {
  types: [
    { value: 'datasource', label: 'Data Sources', disabled: false },
    { value: 'tool', label: 'Tools', disabled: true },
  ],
} as const;

export function SearchFilters({
  open,
  onOpenChange,
  filters,
  onFiltersChange,
}: SearchFiltersProps) {
  const t = useTranslations('search.filterDialog');
  const [localFilters, setLocalFilters] = React.useState<SearchFiltersType>(filters);
  const [organizations, setOrganizations] = React.useState<OrganizationListItem[]>([]);
  const [isLoadingOrgs, setIsLoadingOrgs] = React.useState(false);
  const [orgSearchQuery, setOrgSearchQuery] = React.useState('');
  const [showOrgDropdown, setShowOrgDropdown] = React.useState(false);
  const debouncedOrgSearch = useDebounce(orgSearchQuery, 300);
  const orgInputRef = React.useRef<HTMLInputElement>(null);
  const orgDropdownRef = React.useRef<HTMLDivElement>(null);
  // Cache selected organizations for display
  const [selectedOrgsCache, setSelectedOrgsCache] = React.useState<Map<string, OrganizationListItem>>(new Map());

  const handleOpenChangeInternal = (newOpen: boolean) => {
    onOpenChange(newOpen);
  };

  // Sync local filters when dialog opens
  React.useEffect(() => {
    if (open) {
      setLocalFilters(filters);
      // Fetch organization details for any selected organizations that aren't in cache
      const fetchMissingOrgs = async () => {
        const missingOrgs = filters.organizations?.filter(
          (slug) => !selectedOrgsCache.has(slug)
        ) || [];

        if (missingOrgs.length > 0) {
          try {
            // Fetch all organizations to find the ones we need
            // In a real scenario, you might want to batch this or use a different endpoint
            const response = await listOrganizations({ limit: 100 });
            const newCache = new Map(selectedOrgsCache);
            response.data.forEach((org) => {
              if (missingOrgs.includes(org.slug)) {
                newCache.set(org.slug, org);
              }
            });
            setSelectedOrgsCache(newCache);
          } catch (error) {
            console.error('Failed to fetch organization details:', error);
          }
        }
      };

      fetchMissingOrgs();
    }
  }, [open, filters]);

  // Close dropdown when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        orgDropdownRef.current &&
        !orgDropdownRef.current.contains(event.target as Node) &&
        !orgInputRef.current?.contains(event.target as Node)
      ) {
        setShowOrgDropdown(false);
      }
    };

    if (showOrgDropdown) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [showOrgDropdown]);

  // Fetch organizations based on search query
  React.useEffect(() => {
    const fetchOrganizations = async () => {
      if (!showOrgDropdown || debouncedOrgSearch.length < 1) {
        setOrganizations([]);
        return;
      }

      setIsLoadingOrgs(true);
      try {
        const response = await listOrganizations({
          name_contains: debouncedOrgSearch,
          limit: 10,
        });
        setOrganizations(response.data || []);
      } catch (error) {
        console.error('Failed to fetch organizations:', error);
        setOrganizations([]);
      } finally {
        setIsLoadingOrgs(false);
      }
    };

    fetchOrganizations();
  }, [debouncedOrgSearch, showOrgDropdown]);

  const handleTypeToggle = (type: string) => {
    setLocalFilters((prev) => {
      const types = prev.types || [];
      const newTypes = types.includes(type)
        ? types.filter((t) => t !== type)
        : [...types, type];
      return { ...prev, types: newTypes.length > 0 ? newTypes : undefined };
    });
  };

  const handleAddOrganization = (org: OrganizationListItem) => {
    setLocalFilters((prev) => {
      const orgs = prev.organizations || [];
      if (orgs.includes(org.slug)) return prev; // Already added
      return { ...prev, organizations: [...orgs, org.slug] };
    });
    // Add to cache for display
    setSelectedOrgsCache((prev) => new Map(prev).set(org.slug, org));
    setOrgSearchQuery('');
    setShowOrgDropdown(false);
    orgInputRef.current?.focus();
  };

  const handleRemoveOrganization = (slug: string) => {
    setLocalFilters((prev) => {
      const orgs = prev.organizations?.filter((o) => o !== slug);
      return { ...prev, organizations: orgs?.length ? orgs : undefined };
    });
  };

  // Get organization name from slug for display
  const getOrgName = (slug: string) => {
    const cached = selectedOrgsCache.get(slug);
    return cached?.name || slug.toUpperCase();
  };

  const handleApply = () => {
    onFiltersChange(localFilters);
    onOpenChange(false);
  };

  const handleClear = () => {
    const emptyFilters: SearchFiltersType = {};
    setLocalFilters(emptyFilters);
    onFiltersChange(emptyFilters);
  };

  const handleCancel = () => {
    setLocalFilters(filters);
    onOpenChange(false);
  };

  const getActiveFilterCount = () => {
    let count = 0;
    if (localFilters.types?.length) count += localFilters.types.length;
    if (localFilters.organizations?.length) count += localFilters.organizations.length;
    if (localFilters.dateRange?.from || localFilters.dateRange?.to) count += 1;
    if (localFilters.tags?.length) count += localFilters.tags.length;
    return count;
  };

  const activeCount = getActiveFilterCount();

  if (!open) return null;

  return (
    <DialogPrimitive.Root open={true} onOpenChange={handleOpenChangeInternal} modal>
      <DialogPrimitive.Portal container={typeof document !== 'undefined' ? document.getElementById('portal-root') : null}>
        <DialogPrimitive.Overlay className="fixed inset-0 z-[100] bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <DialogPrimitive.Content
          className={cn(
            "fixed left-[50%] top-[50%] z-[100] grid w-full max-w-md translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg"
          )}
        >
          <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none">
            <CloseIcon className="h-4 w-4" />
            <span className="sr-only">Close</span>
          </DialogPrimitive.Close>

          <div className="flex flex-col space-y-1.5 text-center sm:text-left">
            <div className="flex items-center justify-between">
              <DialogPrimitive.Title className="text-lg font-semibold leading-none tracking-tight">
                {t('title')}
              </DialogPrimitive.Title>
              {activeCount > 0 && (
                <Badge variant="secondary" className="ml-2">
                  {activeCount} {t('active')}
                </Badge>
              )}
            </div>
            <DialogPrimitive.Description className="text-sm text-muted-foreground">
              {t('description')}
            </DialogPrimitive.Description>
          </div>

        <div className="space-y-6 py-4">
          {/* Type Filters */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium">{t('type')}</Label>
              {localFilters.types && localFilters.types.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    setLocalFilters((prev) => ({ ...prev, types: undefined }))
                  }
                  className="h-auto p-0 text-xs text-muted-foreground hover:text-foreground"
                >
                  <CloseIcon className="mr-1 h-3 w-3" />
                  {t('clearSection')}
                </Button>
              )}
            </div>
            <div className="flex flex-col gap-3">
              {FILTER_OPTIONS.types.map((option) => (
                <div key={option.value} className="flex items-center space-x-2">
                  <Checkbox
                    id={`type-${option.value}`}
                    checked={localFilters.types?.includes(option.value) || false}
                    onCheckedChange={() => handleTypeToggle(option.value)}
                    disabled={option.disabled}
                  />
                  <Label
                    htmlFor={`type-${option.value}`}
                    className={cn(
                      "text-sm font-normal",
                      option.disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"
                    )}
                  >
                    {option.label}
                    {option.disabled && <span className="ml-2 text-xs text-muted-foreground">(Coming soon)</span>}
                  </Label>
                </div>
              ))}
            </div>
          </div>

          <Separator />

          {/* Organization Filters */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium">{t('organizations')}</Label>
              {localFilters.organizations && localFilters.organizations.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    setLocalFilters((prev) => ({ ...prev, organizations: undefined }))
                  }
                  className="h-auto p-0 text-xs text-muted-foreground hover:text-foreground"
                >
                  <CloseIcon className="mr-1 h-3 w-3" />
                  {t('clearSection')}
                </Button>
              )}
            </div>

            {/* Search Input */}
            <div className="relative">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  ref={orgInputRef}
                  type="text"
                  placeholder="Search organizations..."
                  value={orgSearchQuery}
                  onChange={(e) => {
                    setOrgSearchQuery(e.target.value);
                    setShowOrgDropdown(true);
                  }}
                  onFocus={() => setShowOrgDropdown(true)}
                  className="pl-9 pr-3 focus-visible:ring-offset-0"
                />
              </div>

              {/* Autocomplete Dropdown */}
              {showOrgDropdown && (orgSearchQuery.length >= 1) && (
                <div
                  ref={orgDropdownRef}
                  className="absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto rounded-md border bg-popover shadow-md"
                >
                  {isLoadingOrgs ? (
                    <div className="flex items-center justify-center py-4">
                      <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                    </div>
                  ) : organizations.length > 0 ? (
                    <div className="py-1">
                      {organizations
                        .filter((org) => !localFilters.organizations?.includes(org.slug))
                        .map((org) => (
                          <button
                            key={org.slug}
                            type="button"
                            onClick={() => handleAddOrganization(org)}
                            className="w-full px-3 py-2 text-left hover:bg-accent transition-colors"
                          >
                            <div className="font-medium text-sm">{org.name}</div>
                            {org.description && (
                              <div className="text-xs text-muted-foreground">{org.description}</div>
                            )}
                          </button>
                        ))}
                      {organizations.every((org) => localFilters.organizations?.includes(org.slug)) && (
                        <div className="px-3 py-2 text-sm text-muted-foreground">
                          All matching organizations already selected
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="px-3 py-2 text-sm text-muted-foreground">
                      No organizations found
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Selected Organizations */}
            {localFilters.organizations && localFilters.organizations.length > 0 && (
              <div className="flex flex-wrap gap-2 pt-2">
                {localFilters.organizations.map((slug) => (
                  <Badge key={slug} variant="secondary" className="gap-1">
                    {getOrgName(slug)}
                    <button
                      type="button"
                      onClick={() => handleRemoveOrganization(slug)}
                      className="ml-1 rounded-full hover:bg-muted"
                    >
                      <CloseIcon className="h-3 w-3" />
                    </button>
                  </Badge>
                ))}
              </div>
            )}
          </div>
        </div>

          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:space-x-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                handleCancel();
              }}
              className="flex-1 sm:flex-none"
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={handleClear}
              disabled={activeCount === 0}
              className="flex-1 sm:flex-none"
            >
              {t('clearAll')}
            </Button>
            <Button
              type="button"
              onClick={() => {
                handleApply();
              }}
              className="flex-1 sm:flex-none"
            >
              {t('apply')}
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
