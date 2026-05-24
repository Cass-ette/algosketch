class Algorithms {
    Node reverse_linked_list(Node head) {
        Node previous = null;
        Node current = head;
        while (current != null) {
            Node next_node = current.next;
            current.next = previous;
            previous = current;
            current = next_node;
        }
        return previous;
    }
}
