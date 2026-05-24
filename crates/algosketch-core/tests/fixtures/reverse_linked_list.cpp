Node reverse_linked_list(Node head) {
    Node previous = nullptr;
    Node current = head;
    while (current != nullptr) {
        Node next_node = current.next;
        current.next = previous;
        previous = current;
        current = next_node;
    }
    return previous;
}
