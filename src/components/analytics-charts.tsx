import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";

type ChartDatum = {
  name: string;
  value: number;
};

type AnalyticsChartsProps = {
  categoryChartData: ChartDatum[];
  sizeChartData: ChartDatum[];
};

const COLORS = [
  "#3b82f6",
  "#10b981",
  "#f59e0b",
  "#f43f5e",
  "#ef4444",
  "#8b5cf6",
  "#6366f1",
  "#64748b",
  "#22d3ee",
  "#f87171",
];

export function AnalyticsCharts({ categoryChartData, sizeChartData }: AnalyticsChartsProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">Files per Category</CardTitle>
        </CardHeader>
        <CardContent className="h-[250px] pb-4">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={categoryChartData}
                cx="50%"
                cy="50%"
                innerRadius={60}
                outerRadius={80}
                paddingAngle={5}
                dataKey="value"
              >
                {categoryChartData.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1e293b",
                  border: "none",
                  borderRadius: "8px",
                  color: "#fff",
                }}
              />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">Storage by Category (MB)</CardTitle>
        </CardHeader>
        <CardContent className="h-[250px] pb-4">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={sizeChartData}>
              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#333" />
              <XAxis
                dataKey="name"
                tick={{ fontSize: 12, fill: "#94a3b8" }}
                tickLine={false}
                axisLine={false}
              />
              <YAxis tick={{ fontSize: 12, fill: "#94a3b8" }} tickLine={false} axisLine={false} />
              <Tooltip
                cursor={{ fill: "#1e293b" }}
                contentStyle={{
                  backgroundColor: "#1e293b",
                  border: "none",
                  borderRadius: "8px",
                  color: "#fff",
                }}
              />
              <Bar dataKey="value" fill="#8b5cf6" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
    </div>
  );
}
