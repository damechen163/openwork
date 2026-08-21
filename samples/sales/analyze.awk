BEGIN {
    FS = ","
    output_directory = ENVIRON["OPENWORK_OUTPUT_DIR"]
    if (output_directory == "") {
        output_directory = "/workspace/output"
    }
    csv_output = output_directory "/sales-analysis.csv"
    summary_output = output_directory "/summary.md"
}

FNR == 1 {
    if ($0 != "customer_id,customer_name,sales,orders") {
        exit 20
    }
    next
}

NR == FNR {
    if (NF != 4 || seen_july[$1]++) {
        exit 21
    }
    ids[++customer_count] = $1
    customer_name[$1] = $2
    july_sales[$1] = $3 + 0
    july_orders[$1] = $4 + 0
    next
}

{
    if (NF != 4 || seen_august[$1]++ || !seen_july[$1] || customer_name[$1] != $2) {
        exit 22
    }
    august_sales[$1] = $3 + 0
    august_orders[$1] = $4 + 0
}

function rounded_basis_points(change, baseline, magnitude, rounded) {
    magnitude = change < 0 ? -change : change
    rounded = int((magnitude * 10000 + int(baseline / 2)) / baseline)
    return change < 0 ? -rounded : rounded
}

function format_rate(basis_points, sign, magnitude) {
    sign = basis_points < 0 ? "-" : ""
    magnitude = basis_points < 0 ? -basis_points : basis_points
    return sprintf("%s%d.%02d%%", sign, int(magnitude / 100), magnitude % 100)
}

END {
    if (customer_count == 0) {
        exit 23
    }
    for (row_index = 1; row_index <= customer_count; row_index++) {
        id = ids[row_index]
        if (!seen_august[id] || july_sales[id] <= 0 || july_orders[id] <= 0 || august_sales[id] <= 0 || august_orders[id] <= 0) {
            exit 24
        }
        sales_change[id] = august_sales[id] - july_sales[id]
        sales_decline[id] = -sales_change[id]
        sales_growth[id] = rounded_basis_points(sales_change[id], july_sales[id])
        order_change[id] = august_orders[id] - july_orders[id]
        july_sales_total += july_sales[id]
        august_sales_total += august_sales[id]
        july_orders_total += july_orders[id]
        august_orders_total += august_orders[id]
    }

    for (left = 1; left < customer_count; left++) {
        for (right = left + 1; right <= customer_count; right++) {
            left_id = ids[left]
            right_id = ids[right]
            if (sales_decline[right_id] > sales_decline[left_id] || (sales_decline[right_id] == sales_decline[left_id] && right_id < left_id)) {
                ids[left] = right_id
                ids[right] = left_id
            }
        }
    }

    print "customer_id,customer_name,july_sales,july_orders,august_sales,august_orders,sales_change,sales_decline,sales_growth_rate,order_change" > csv_output
    for (row_index = 1; row_index <= customer_count; row_index++) {
        id = ids[row_index]
        printf "%s,%s,%d,%d,%d,%d,%d,%d,%s,%d\n", id, customer_name[id], july_sales[id], july_orders[id], august_sales[id], august_orders[id], sales_change[id], sales_decline[id], format_rate(sales_growth[id]), order_change[id] >> csv_output
    }
    total_sales_change = august_sales_total - july_sales_total
    total_order_change = august_orders_total - july_orders_total
    total_sales_growth = rounded_basis_points(total_sales_change, july_sales_total)
    total_order_growth = rounded_basis_points(total_order_change, july_orders_total)
    printf "TOTAL,,%d,%d,%d,%d,%d,%d,%s,%d\n", july_sales_total, july_orders_total, august_sales_total, august_orders_total, total_sales_change, -total_sales_change, format_rate(total_sales_growth), total_order_change >> csv_output
    close(csv_output)

    largest = ids[1]
    print "# Sales comparison" > summary_output
    print "" >> summary_output
    printf "- July sales total: %d\n", july_sales_total >> summary_output
    printf "- August sales total: %d\n", august_sales_total >> summary_output
    printf "- Sales change: %d (%s)\n", total_sales_change, format_rate(total_sales_growth) >> summary_output
    printf "- July order count: %d\n", july_orders_total >> summary_output
    printf "- August order count: %d\n", august_orders_total >> summary_output
    printf "- Order change: %d (%s)\n", total_order_change, format_rate(total_order_growth) >> summary_output
    printf "- Largest sales decline: %s (%s), %d (%s)\n", customer_name[largest], largest, sales_decline[largest], format_rate(sales_growth[largest]) >> summary_output
    printf "- Acme sales decline: %d\n", sales_decline["C001"] >> summary_output
    printf "- Beta sales growth: %d\n", sales_change["C002"] >> summary_output
    printf "- Delta sales change: %d\n", sales_change["C004"] >> summary_output
    close(summary_output)
}
