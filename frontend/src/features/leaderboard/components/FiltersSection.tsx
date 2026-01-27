import { useState, useEffect } from "react";
import { ChevronDown } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { getEcosystems } from "../../../shared/api/client";
import { FilterType } from "../types";

interface FiltersSectionProps {
  activeFilter: FilterType;
  onFilterChange: (filter: FilterType) => void;
  selectedEcosystem: EcosystemOption;
  onEcosystemChange: (ecosystem: EcosystemOption) => void;
  showDropdown: boolean;
  onToggleDropdown: () => void;
  isLoaded: boolean;
}

interface EcosystemOption {
  label: string;
  value: string;
}

interface FilterOption {
  label: string;
  value: FilterType;
}

export function FiltersSection({
  activeFilter,
  onFilterChange,
  selectedEcosystem,
  onEcosystemChange,
  showDropdown,
  onToggleDropdown,
  isLoaded,
}: FiltersSectionProps) {
  const { theme } = useTheme();

  const [ecosystemOptions, setEcosystemOptions] = useState<EcosystemOption[]>([
    { label: "All Ecosystems", value: "all" },
  ]);
  const [loading, setLoading] = useState(false);
  const [showFilterDropdown, setShowFilterDropdown] = useState(false);

  // Define filter options
  const filterOptions: FilterOption[] = [
    { label: "Overall Leaderboard", value: "overall" },
    { label: "Total Rewards", value: "rewards" },
    { label: "Total Contributions", value: "contributions" },
  ];

  // Get the label for the currently active filter
  const getActiveFilterLabel = () => {
    const activeOption = filterOptions.find(
      (option) => option.value === activeFilter
    );
    return activeOption?.label || "Overall Leaderboard";
  };

  useEffect(() => {
    const fetchEcosystems = async () => {
      try {
        setLoading(true);
        const data = await getEcosystems();

        const activeEcosystems = data.ecosystems
          .filter((e) => e.status === "active")
          .map((e) => ({
            label: e.name,
            value: e.slug,
          }));

        setEcosystemOptions([
          { label: "All Ecosystems", value: "all" },
          ...activeEcosystems,
        ]);
      } catch (err) {
        console.error("Failed to fetch ecosystems:", err);
      } finally {
        setLoading(false);
      }
    };

    fetchEcosystems();
  }, []);

  return (
    <div
      className={`backdrop-blur-[40px] bg-white/[0.12] rounded-[20px] border border-white/20 shadow-[0_4px_16px_rgba(0,0,0,0.06)] p-5 transition-all duration-700 delay-900 relative z-50 ${
        isLoaded ? "opacity-100 translate-y-0" : "opacity-0 translate-y-8"
      }`}
    >
      <div className="flex items-center justify-end flex-wrap gap-4">
        {/* Filter Dropdown Button */}
        <div className="relative z-[100]">
          <button
            onClick={() => setShowFilterDropdown(!showFilterDropdown)}
            className="flex items-center gap-2 px-4 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white border border-white/10 hover:scale-105 transition-all duration-300 shadow-[0_4px_16px_rgba(201,152,58,0.35)]"
          >
            <span
              className="text-[13px] font-semibold text-white"
            >
              {getActiveFilterLabel()}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 text-white ${
                showFilterDropdown ? "rotate-180" : ""
              }`}
            />
          </button>
          {showFilterDropdown && (
            <div className="absolute right-0 mt-2 w-[220px] backdrop-blur-[40px] bg-white/[0.18] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in">
              {filterOptions.map((option) => (
                <button
                  key={option.value}
                  onClick={() => {
                    onFilterChange(option.value);
                    setShowFilterDropdown(false);
                  }}
                  className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                    activeFilter === option.value
                      ? `bg-white/[0.15] font-bold hover:bg-white/[0.25]`
                      : "hover:bg-white/[0.2]"
                  } ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                >
                  {option.label}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Ecosystem Dropdown Button */}
        <div className="relative z-[100]">
          <button
            onClick={onToggleDropdown}
            className="flex items-center gap-2 px-4 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white border border-white/10 hover:scale-105 transition-all duration-300 shadow-[0_4px_16px_rgba(201,152,58,0.35)]"
          >
            <span
              className="text-[13px] font-semibold text-white"
            >
              {selectedEcosystem.label}
            </span>
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 text-white ${
                showDropdown ? "rotate-180" : ""
              }`}
            />
          </button>
          {showDropdown && (
            <div className="absolute right-0 mt-2 w-[200px] backdrop-blur-[40px] bg-white/[0.18] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-[100] animate-dropdown-in">
              {loading ? (
                <div className="px-4 py-3 flex justify-center">
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                </div>
              ) : (
                ecosystemOptions.map((eco, index) => (
                  <button
                    key={eco.value}
                    onClick={() => {
                      onEcosystemChange({ label: eco.label, value: eco.value });
                      onToggleDropdown();
                    }}
                    className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                      index === 0
                        ? `bg-white/[0.15] font-bold hover:bg-white/[0.25]`
                        : "hover:bg-white/[0.2]"
                    } ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                  >
                    {eco.label}
                  </button>
                ))
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}