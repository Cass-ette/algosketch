class Algorithms {
    int[] quick_sort(int[] items, int low, int high) {
        if (low < high) {
            int pivot = partition(items, low, high);
            quick_sort(items, low, pivot - 1);
            quick_sort(items, pivot + 1, high);
        }
        return items;
    }

    int partition(int[] items, int low, int high) {
        int pivot = items[high];
        int i = low;
        for (int j = low; j < high; j = j + 1) {
            if (items[j] < pivot) {
                int temp = items[i];
                items[i] = items[j];
                items[j] = temp;
                i = i + 1;
            }
        }
        int temp = items[i];
        items[i] = items[high];
        items[high] = temp;
        return i;
    }
}
