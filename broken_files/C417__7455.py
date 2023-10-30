class Item():
        if ((item := item.capitalize()) in
            (item_list := list(map(lambda item_tuple: item_tuple[0],
                                   search_results := _search_item_data(item))))):
            self.__id = self.__item_data[1]